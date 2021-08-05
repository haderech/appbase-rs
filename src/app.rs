use std::any::TypeId;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use futures::lock::Mutex as FutureMutex;
use once_cell::sync::Lazy;
use serde_json::value::Value;
use tokio::sync::{broadcast, watch};

use crate::plugin::{Plugin, PluginState};

static mut APP: Lazy<Application> = Lazy::new(|| {
   Application::new()
});

pub type PluginHandle = Arc<Mutex<dyn Plugin>>;
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
   plugins: HashMap<TypeId, PluginMeta>,
   running_plugins: Vec<TypeId>,
   channels: HashMap<String, ChannelHandle>,
   is_quiting: AtomicBool,
   quit_tx: watch::Sender<bool>,
   quit_rx: Option<watch::Receiver<bool>>,
}

struct PluginMeta {
   instance: PluginHandle,
   state: PluginState,
}

impl PluginMeta {
   fn new<P>() -> PluginMeta where P: Plugin {
      PluginMeta {
         instance: Arc::new(Mutex::new(P::new())),
         state: PluginState::Registered,
      }
   }
}

impl Application {
   fn new() -> Application {
      let (tx, rx) = watch::channel(false);
      Application {
         plugins: HashMap::new(),
         running_plugins: Vec::new(),
         channels: HashMap::new(),
         is_quiting: AtomicBool::new(false),
         quit_tx: tx,
         quit_rx: Some(rx),
      }
   }

   fn initialize(&mut self) {
      for plugin in self.plugins.values_mut() {
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
         if let Ok(mut p1) = plugin.instance.lock() {
            p1.plugin_startup();
         }
      }
   }

   async fn execute(&mut self) {
      let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
      let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();
      tokio::select! {
         _ = sigint.recv() => {},
         _ = sigterm.recv() => {},
      }
      self.shutdown().await;
   }

   fn quit(&mut self) {
      let _ = self.quit_tx.send(true);
      self.quit_rx.take();
      self.is_quiting.store(true, Ordering::Relaxed);
   }

   async fn shutdown(&mut self) {
      self.quit();
      self.quit_tx.closed().await;
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
   }
}

pub fn initialize() {
   unsafe {
      APP.initialize();
   }
}

pub fn startup() {
   unsafe {
      APP.startup();
   }
}


pub async fn execute() {
   unsafe {
      APP.execute().await;
   }
}

pub fn quit() {
   unsafe {
      APP.quit();
   }
}

pub fn plugin_initialized<P>() -> bool where P: Plugin {
   unsafe {
      if let Some(plugin) = APP.plugins.get_mut(&TypeId::of::<P>()) {
         if plugin.state == PluginState::Registered {
            plugin.state = PluginState::Initialized;
            return true;
         }
      }
      false
   }
}

pub fn plugin_started<P>() -> bool where P: Plugin {
   unsafe {
      let typeid = TypeId::of::<P>();
      if let Some(plugin) = APP.plugins.get_mut(&typeid) {
         if plugin.state == PluginState::Initialized {
            plugin.state = PluginState::Started;
            APP.running_plugins.push(typeid);
            return true;
         }
      }
      false
   }
}

pub fn register_plugin<P>() where P: Plugin {
   unsafe {
      let typeid = TypeId::of::<P>();
      if !APP.plugins.contains_key(&typeid) {
         APP.plugins.insert(typeid, PluginMeta::new::<P>());
      }
   }
}

pub fn get_plugin<P>() -> PluginHandle where P: Plugin {
   unsafe {
      match find_plugin(TypeId::of::<P>()) {
         Some(plugin) => plugin,
         None => {
            let typeid = TypeId::of::<P>();
            APP.plugins.insert(typeid, PluginMeta::new::<P>());
            APP.plugins.get(&typeid).unwrap().instance.clone()
         }
      }
   }
}

pub fn find_plugin(typeid: TypeId) -> Option<PluginHandle> {
   unsafe {
      match APP.plugins.get(&typeid) {
         Some(plugin) => Some(plugin.instance.clone()),
         None => None
      }
   }
}

pub fn get_channel(name: String) -> ChannelHandle {
   unsafe {
      match APP.channels.get(name.as_str()) {
         Some(channel) => Arc::clone(channel),
         None => {
            let (tx, _) = broadcast::channel(16);
            let _name = name.clone();
            APP.channels.insert(_name, Arc::new(Mutex::new(tx)));
            Arc::clone(APP.channels.get(name.as_str()).unwrap())
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
