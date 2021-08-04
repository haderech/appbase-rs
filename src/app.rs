use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use futures::lock::Mutex as FutureMutex;
use jsonrpc_core::Value;
use once_cell::sync::Lazy;
use tokio::signal;
use tokio::sync::broadcast::{channel, Receiver, Sender};

use crate::plugin::Plugin;

static mut APP: Lazy<Application> = Lazy::new(|| {
   Application::new()
});

pub type PluginHandle = Arc<Mutex<dyn Plugin>>;
pub type ChannelHandle = Arc<Mutex<Sender<Value>>>;
pub type SubscribeHandle = Arc<FutureMutex<Receiver<Value>>>;

struct Application {
   plugins: HashMap<String, PluginHandle>,
   running_plugins: Vec<PluginHandle>,
   channels: HashMap<String, ChannelHandle>,
   pub is_quiting: AtomicBool,
}

impl Application {
   fn new() -> Application {
      Application {
         plugins: HashMap::new(),
         running_plugins: Vec::new(),
         channels: HashMap::new(),
         is_quiting: AtomicBool::new(false),
      }
   }

   fn initialize(&mut self) {
      for plugin in self.plugins.values() {
         plugin.lock().unwrap().initialize();
      }
   }

   fn startup(&mut self) {
      if self.is_quiting.load(Ordering::Relaxed) {
         return;
      }
      for plugin in self.plugins.values() {
         plugin.lock().unwrap().startup();
      }
   }

   async fn execute(&mut self) {
      //loop {}
      signal::ctrl_c().await.unwrap();
      self.shutdown().await;
   }

   fn quit(&mut self) {
      self.is_quiting.store(true, Ordering::Relaxed);
   }

   async fn shutdown(&mut self) {
      self.quit();
      for plugin in self.running_plugins.iter().rev() {
         plugin.lock().unwrap().shutdown();
         if let Some(handle) = plugin.lock().unwrap().handle() {
            let _ = handle.await;
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

pub fn plugin_started<P>() where P: Plugin {
   unsafe {
      let plugin = get_plugin::<P>();
      APP.running_plugins.push(plugin.clone());
   }
}

pub fn register_plugin<P>() where P: Plugin {
   unsafe {
      if !APP.plugins.contains_key(P::typename().as_str()) {
         APP.plugins.insert(P::typename(), Arc::new(Mutex::new(P::new())));
      }
   }
}

pub fn get_plugin<P>() -> PluginHandle where P: Plugin {
   unsafe {
      match find_plugin(P::typename()) {
         Some(plugin) => plugin,
         None => {
            APP.plugins.insert(P::typename(), Arc::new(Mutex::new(P::new())));
            Arc::clone(APP.plugins.get(P::typename().as_str()).unwrap())
         }
      }
   }
}

pub fn find_plugin(name: String) -> Option<PluginHandle> {
   unsafe {
      match APP.plugins.get(name.as_str()) {
         Some(plugin) => Some(Arc::clone(plugin)),
         None => None
      }
   }
}

pub fn get_channel(name: String) -> ChannelHandle {
   unsafe {
      match APP.channels.get(name.as_str()) {
         Some(channel) => Arc::clone(channel),
         None => {
            let (tx, _) = channel(32);
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

pub fn is_quiting() -> bool {
   unsafe {
      APP.is_quiting.load(Ordering::Relaxed)
   }
}