use std::collections::HashMap;
use std::future::Future;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;
use serde_json::value::Value;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::{broadcast, Mutex as FutureMutex, watch};
use tokio::task::JoinHandle;

use crate::plugin::{Plugin as PluginImpl, PluginState};

static mut APP: Lazy<Application> = Lazy::new(|| {
   Application::new()
});

pub type PluginHandle = Arc<Mutex<dyn PluginImpl>>;
pub type ChannelHandle = Arc<Mutex<broadcast::Sender<Value>>>;
pub type SubscribeHandle = Arc<FutureMutex<broadcast::Receiver<Value>>>;

pub struct QuitHandle {
   handle: watch::Receiver<bool>,
}

impl QuitHandle {
   pub fn is_quiting(&self) -> bool {
      *self.handle.borrow()
   }
}

struct Application {
   runtime: Arc<Runtime>,
   plugins: HashMap<String, Plugin>,
   running_plugins: Vec<String>,
   channels: HashMap<String, ChannelHandle>,
   is_quiting: AtomicBool,
   quit_tx: Option<watch::Sender<bool>>,
   quit_rx: Option<watch::Receiver<bool>>,
}

struct Plugin {
   instance: PluginHandle,
   state: PluginState,
}

impl Plugin {
   fn new<P>() -> Plugin where P: PluginImpl {
      Plugin {
         instance: Arc::new(Mutex::new(P::new())),
         state: PluginState::Registered,
      }
   }
}

impl Application {
   fn new() -> Application {
      let (tx, rx) = watch::channel(false);
      Application {
         runtime: Arc::new(Builder::new_multi_thread().enable_all().build().unwrap()),
         plugins: HashMap::new(),
         running_plugins: Vec::new(),
         channels: HashMap::new(),
         is_quiting: AtomicBool::new(false),
         quit_tx: Some(tx),
         quit_rx: Some(rx),
      }
   }

   fn initialize<P>(&mut self) where P: PluginImpl {
      if let Some(plugin) = self.plugins.get(P::type_name()) {
         if let Ok(mut p1) = plugin.instance.lock() {
            p1.plugin_initialize();
         }
      }
   }

   fn startup(&mut self) {
      if self.is_quiting.load(Ordering::Relaxed) {
         return;
      }
      for plugin in self.plugins.values_mut() {
         if plugin.state != PluginState::Initialized {
            continue;
         }
         if let Ok(mut p1) = plugin.instance.lock() {
            p1.plugin_startup();
         }
      }
   }

   fn execute(&mut self) {
      self.runtime.block_on(async {
         let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
         let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
         tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
         }
      });
      self.shutdown();
   }

   fn quit(&mut self) {
      let _ = self.quit_tx.as_ref().unwrap().send(true);
      self.quit_rx.take();
      self.is_quiting.store(true, Ordering::Relaxed);
   }

   fn shutdown(&mut self) {
      self.quit();
      let quit_tx = self.quit_tx.take().unwrap();
      self.runtime.clone().block_on(async {
         quit_tx.closed().await;
         for typeid in self.running_plugins.iter().rev() {
            let plugin = self.plugins.get_mut(typeid).unwrap();
            if let Ok(mut p1) = plugin.instance.lock() {
               if plugin.state != PluginState::Started {
                  continue;
               }
               plugin.state = PluginState::Stopped;
               p1.plugin_shutdown().await;
            }
         }
      });
   }
}

pub fn initialize_plugin<P>() where P: PluginImpl {
   unsafe {
      APP.initialize::<P>();
   }
}

#[macro_export]
macro_rules! initialize {
   ($($plugin:ty),*) => {
      $(::appbase::app::initialize_plugin::<$plugin>();)*
   };
}

pub use initialize;

pub fn startup() {
   unsafe {
      APP.startup();
   }
}

pub fn execute() {
   unsafe {
      APP.execute();
   }
}

pub fn quit() {
   unsafe {
      APP.quit();
   }
}

pub fn plugin_initialized<P>() -> bool where P: PluginImpl {
   unsafe {
      if let Some(plugin) = APP.plugins.get_mut(P::type_name()) {
         if plugin.state == PluginState::Registered {
            plugin.state = PluginState::Initialized;
            return false;
         }
      }
      true
   }
}

pub fn plugin_started<P>() -> bool where P: PluginImpl {
   unsafe {
      let type_name = P::type_name();
      if let Some(plugin) = APP.plugins.get_mut(type_name) {
         if plugin.state == PluginState::Initialized {
            plugin.state = PluginState::Started;
            APP.running_plugins.push(type_name.to_string());
            return false;
         }
      }
      true
   }
}

pub fn register_plugin<P>() where P: PluginImpl {
   unsafe {
      let type_name = P::type_name();
      if !APP.plugins.contains_key(type_name) {
         APP.plugins.insert(type_name.to_string(), Plugin::new::<P>());
         APP.plugins.get(type_name).unwrap().instance.lock().unwrap().resolve_deps();
      }
   }
}

pub fn get_plugin<P>() -> PluginHandle where P: PluginImpl {
   find_plugin(P::type_name()).unwrap()
}

pub fn find_plugin(type_name: &str) -> Option<PluginHandle> {
   unsafe {
      match APP.plugins.get(type_name) {
         Some(plugin) => Some(plugin.instance.clone()),
         None => None
      }
   }
}

pub fn get_channel(name: String) -> ChannelHandle {
   unsafe {
      match APP.channels.get(name.as_str()) {
         Some(channel) => channel.clone(),
         None => {
            let (tx, _) = broadcast::channel(16);
            let _name = name.clone();
            APP.channels.insert(_name, Arc::new(Mutex::new(tx)));
            APP.channels.get(name.as_str()).unwrap().clone()
         }
      }
   }
}

pub fn subscribe_channel(name: String) -> SubscribeHandle {
   Arc::new(FutureMutex::new(get_channel(name).lock().unwrap().subscribe()))
}

pub fn quit_handle() -> Option<QuitHandle> {
   unsafe {
      if let Some(rx) = APP.quit_rx.as_ref() {
         return Some(QuitHandle{ handle: rx.clone() });
      }
      None
   }
}

pub fn block_on<F: Future<Output=()>>(future: F) -> F::Output {
   unsafe {
      APP.runtime.block_on(future);
   }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output> where F: Future + Send + 'static, F::Output: Send + 'static {
   unsafe {
      APP.runtime.spawn(future)
   }
}

pub fn spawn_blocking<F, R>(func: F) -> JoinHandle<R> where F: FnOnce() -> R + Send + 'static, R: Send + 'static {
   unsafe {
      APP.runtime.spawn_blocking(func)
   }
}
