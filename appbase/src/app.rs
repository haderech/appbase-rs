use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

use channels::Channels;
use options::Options;

use crate::plugin::{Plugin, State};
use crate::util::current_exe;

mod options;
mod channels;

pub static APP: Lazy<App> = Lazy::new(|| {
   App::new()
});

pub type Plugins = HashMap<String, Mutex<Option<Box<dyn Plugin>>>>;

pub struct App {
   runtime: Runtime,
   plugins: RwLock<Plugins>,
   plugin_states: RwLock<HashMap<String, State>>,
   running_plugins: Mutex<Vec<String>>,
   pub channels: Channels,
   pub options: Options,
   is_quitting: RwLock<Option<Arc<AtomicBool>>>,
}

impl App {
   pub fn new() -> Self {
      App {
         runtime: Builder::new_multi_thread().enable_all().build().unwrap(),
         plugins: RwLock::new(HashMap::new()),
         plugin_states: RwLock::new(HashMap::new()),
         running_plugins: Mutex::new(Vec::new()),
         channels: Channels::new(),
         is_quitting: RwLock::new(Some(Arc::new(AtomicBool::new(false)))),
         options: Options::new(&current_exe()),
      }
   }

   pub fn is_registered<P: Plugin>(&self) -> bool {
      self.plugins.try_read().unwrap().contains_key(P::type_name())
   }

   pub fn register<P: Plugin>(&self) {
      if self.is_registered::<P>() {
         return;
      } else {
         // temporary add None with plugin name to prevent recursive registration
         self.plugins.try_write().unwrap().insert(String::from(P::type_name()), Mutex::new(None));
         self.plugin_states.try_write().unwrap().insert(String::from(P::type_name()), State::Registered);
      }
      let p = Box::new(P::new());
      p.resolve_deps();
      self.plugins.try_write().unwrap().get_mut(P::type_name()).unwrap().try_lock().unwrap().replace(p);
   }

   fn _plugin_init(&self, name: &str) {
      if !self.options.is_parsed() {
         self.init();
      }
      match self.plugins.try_read() {
         Ok(plugins) => {
            plugins.get(name).unwrap().try_lock().unwrap().as_mut().unwrap()._init();
         },
         Err(_) => panic!("locked: plugins"),
      }
   }

   pub fn plugin_init<P: Plugin>(&self) {
      self._plugin_init(P::type_name());
   }

   pub fn plugin_startup<P: Plugin>(&self) {
      match self.plugins.try_read() {
         Ok(plugins) => {
            plugins.get(P::type_name()).unwrap().try_lock().unwrap().as_mut().unwrap()._startup();
         },
         Err(_) => panic!("locked: plugins"),
      }
   }

   pub fn init(&self) {
      self.options.parse();
      if let Some(names) = self.options.values_of("app::plugin") {
         for name in names {
            self._plugin_init(&name);
         }
      }
   }

   pub fn startup(&self) {
      {
         let is_quitting = self.is_quitting.try_read().unwrap();
         if is_quitting.is_none() || is_quitting.as_ref().unwrap().load(Ordering::Relaxed) {
            log::warn!("cannot start closing app...");
            return;
         }
      }
      match self.plugins.try_read() {
         Ok(plugins) => {
            let mut itr = plugins.iter();
            while let Some((k, p)) = itr.next() {
               if *self.plugin_states.try_read().unwrap().get(k).unwrap() == State::Initialized {
                  p.try_lock().unwrap().as_mut().unwrap()._startup();
               }
            }
         },
         Err(_) => panic!("locked: plugins"),
      }
   }

   pub fn execute(&self) {
      if self.running_plugins.try_lock().unwrap().len() == 0 {
         return;
      }
      self.runtime.block_on(async {
         let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
         let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();

         let is_quitting = self.is_quitting.try_read().unwrap().as_ref().unwrap().clone();
         let main_loop = self.runtime.spawn_blocking(move || {
            loop {
               if is_quitting.load(Ordering::Relaxed) {
                  break;
               }
            }
         });
         tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
            _ = main_loop => {},
         }
      });
      self.shutdown();
   }

   pub fn quit(&self) {
      let is_quitting = self.is_quitting.try_read().unwrap();
      if let Some(is_quitting) = is_quitting.as_ref() {
         is_quitting.store(true, Ordering::Relaxed);
      }
   }

   fn shutdown(&self) {
      self.quit();
      let is_quitting = self.is_quitting.write().unwrap().take().unwrap();
      self.runtime.block_on(async {
         log::info!("try graceful shutdown...");
         while Arc::strong_count(&is_quitting) > 1 {
         }
         for name in self.running_plugins.try_lock().unwrap().iter().rev() {
            if let Some(p) = self.plugins.try_read().unwrap().get(name).unwrap().try_lock().unwrap().as_mut() {
               p._shutdown();
            }
         }
      });
   }

   pub fn run_with<P: Plugin, R, F>(&self, f: F) -> R where F: FnOnce(&mut P) -> R {
      let plugins = self.plugins.try_read().unwrap();
      let mut p = plugins.get(P::type_name()).unwrap().try_lock().unwrap();
      f(p.as_mut().unwrap().downcast_mut::<P>().unwrap())
   }

   pub fn state_of<P: Plugin>(&self) -> State {
      self.plugin_states.try_read().unwrap().get(P::type_name()).unwrap().clone()
   }

   pub fn set_state_of<P: Plugin>(&self, state: State) {
      let mut states = self.plugin_states.try_write().unwrap();
      let s = states.get_mut(P::type_name()).unwrap();
      *s = state;
      if state == State::Started {
         self.running_plugins.try_lock().unwrap().push(String::from(P::type_name()));
      }
   }

   pub fn spawn<F>(&self, future: F) -> tokio::task::JoinHandle<F::Output> where F: std::future::Future + Send + 'static, F::Output: Send + 'static {
      self.runtime.spawn(future)
   }

   pub fn spawn_blocking<F, R>(&self, func: F) -> tokio::task::JoinHandle<R> where F: FnOnce() -> R + Send + 'static, R: Send + 'static {
      self.runtime.spawn_blocking(func)
   }

   pub fn quit_handle(&self) -> Option<QuitHandle> {
      let is_quitting = self.is_quitting.try_read().unwrap();
      if is_quitting.is_some() && !is_quitting.as_ref().unwrap().load(Ordering::Relaxed) {
         return Some(QuitHandle {
            is_quitting: is_quitting.as_ref().unwrap().clone(),
         });
      }
      None
   }
}

pub struct QuitHandle {
   is_quitting: Arc<AtomicBool>,
}

impl QuitHandle {
   pub fn is_quiting(&self) -> bool {
      self.is_quitting.load(Ordering::Relaxed)
   }
   pub fn quit(&mut self) {
      self.is_quitting.store(true, Ordering::Relaxed)
   }
}
