use std::collections::HashMap;
use std::sync::{Arc, Mutex, RwLock};
use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;
use tokio::runtime::{Builder, Runtime};

use channels::Channels;
use options::Options;

use crate::plugin::{Plugin, State};
use crate::util::current_exe;

pub mod options;
pub mod channels;

pub static APP: Lazy<App> = Lazy::new(|| {
   App::new()
});

pub type Plugins = HashMap<String, Mutex<Option<Box<dyn Plugin>>>>;

pub struct App {
   runtime: RwLock<Option<Runtime>>,
   plugins: RwLock<Plugins>,
   plugin_states: RwLock<HashMap<String, State>>,
   running_plugins: Mutex<Vec<String>>,
   pub channels: Channels,
   pub options: Options,
   is_quitting: Arc<AtomicBool>,
}

impl App {
   pub fn new() -> Self {
      App {
         runtime: RwLock::new(None),
         plugins: RwLock::new(HashMap::new()),
         plugin_states: RwLock::new(HashMap::new()),
         running_plugins: Mutex::new(Vec::new()),
         channels: Channels::new(),
         is_quitting: Arc::new(AtomicBool::new(false)),
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
      let mut runtime = self.runtime.try_write().unwrap();
      let mut builder = Builder::new_multi_thread();
      if let Some(worker_threads) = self.options.value_of_t::<usize>("app::worker-threads") {
         builder.worker_threads(worker_threads);
      }
      if let Some(max_blocking_threads) = self.options.value_of_t::<usize>("app::max-blocking-threads") {
         builder.max_blocking_threads(max_blocking_threads);
      }
      runtime.replace(builder.enable_all().build().unwrap());
      if let Some(capacity) = self.options.value_of_t::<usize>("app::channel-capacity") {
         self.channels.set_capacity(capacity);
      }
      if let Some(names) = self.options.values_of("app::plugin") {
         for name in names {
            self._plugin_init(&name);
         }
      }
   }

   pub fn startup(&self) {
      if self.is_quitting.load(Ordering::Acquire) {
         log::warn!("cannot start closing app...");
         return;
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
      self.runtime.try_read().unwrap().as_ref().unwrap().block_on(async {
         use tokio::signal::unix::*;

         let mut sigint = signal(SignalKind::interrupt()).unwrap();
         let mut sigterm = signal(SignalKind::terminate()).unwrap();

         let is_quitting = self.is_quitting.clone();
         let main_loop = self.runtime.try_read().unwrap().as_ref().unwrap().spawn_blocking(move || {
            loop {
               if is_quitting.load(Ordering::Acquire) {
                  break;
               }
               std::thread::sleep(std::time::Duration::from_secs(1));
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
      self.is_quitting.store(true, Ordering::Release);
   }

   fn shutdown(&self) {
      self.quit();
      self.runtime.try_read().unwrap().as_ref().unwrap().block_on(async {
         log::info!("try graceful shutdown...");
         while Arc::strong_count(&self.is_quitting) > 1 {
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
      self.runtime.try_read().unwrap().as_ref().unwrap().spawn(future)
   }

   pub fn spawn_blocking<F, R>(&self, func: F) -> tokio::task::JoinHandle<R> where F: FnOnce() -> R + Send + 'static, R: Send + 'static {
      self.runtime.try_read().unwrap().as_ref().unwrap().spawn_blocking(func)
   }

   pub fn quit_handle(&self) -> Option<QuitHandle> {
      (!self.is_quitting.load(Ordering::Acquire)).then_some(QuitHandle { is_quitting: Some(self.is_quitting.clone()) })
   }
}

pub struct QuitHandle {
   is_quitting: Option<Arc<AtomicBool>>,
}

impl QuitHandle {
   pub fn is_quitting(&self) -> bool {
      match &self.is_quitting {
         Some(q) => q.load(Ordering::Acquire),
         None => true,
      }
   }
   pub fn quit(&mut self) {
      self.is_quitting.take().map(|q| q.store(true, Ordering::Release));
   }
}

impl Drop for QuitHandle {
   fn drop(&mut self) {
      self.is_quitting.take();
   }
}
