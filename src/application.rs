use crate::plugin::Plugin;

use futures::lock::Mutex as FutureMutex;
use jsonrpc_core::{Value};
use once_cell::sync::Lazy;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tokio::sync::broadcast::{channel, Sender, Receiver};
use tokio::signal;

pub static mut APP: Lazy<Application> = Lazy::new(|| {
   Application::new()
});

pub type PluginHandle = Arc<Mutex<dyn Plugin>>;
pub type ChannelHandle = Arc<Mutex<Sender<Value>>>;
pub type SubscribeHandle = Arc<FutureMutex<Receiver<Value>>>;

pub struct Application {
   plugins: HashMap<String, PluginHandle>,
   running_plugins: Vec<PluginHandle>,
   channels: HashMap<String, ChannelHandle>,
   is_quiting: bool, // XXX: need mutex?
}

impl Application {
   pub fn new() -> Application {
      Application {
         plugins: HashMap::new(),
         running_plugins: Vec::new(),
         channels: HashMap::new(),
         is_quiting: false,
      }
   }

   pub fn initialize(&mut self) {
      for plugin in self.plugins.values() {
         plugin.lock().unwrap().initialize();
      }
   }

   pub fn startup(&mut self) {
      if self.is_quiting {
         return;
      }
      for plugin in self.plugins.values() {
         plugin.lock().unwrap().startup();
      }
   }

   pub async fn execute(&mut self) {
      //loop {}
      signal::ctrl_c().await.unwrap();
      self.shutdown();
   }

   pub fn quit(&mut self) {
      self.is_quiting = true;
   }

   fn shutdown(&mut self) {
      for plugin in self.running_plugins.iter().rev() {
         plugin.lock().unwrap().shutdown();
      }
   }

   pub fn plugin_started<P>(&mut self) where P: Plugin {
      let plugin = self.get_plugin::<P>();
      self.running_plugins.push(plugin.clone());
   }

   pub fn register_plugin<P>(&mut self) where P: Plugin {
     if !self.plugins.contains_key(P::typename().as_str()) {
        self.plugins.insert(P::typename(), Arc::new(Mutex::new(P::new())));
     }
   }

   pub fn get_plugin<P>(&mut self) -> PluginHandle where P: Plugin {
      match self.find_plugin(P::typename()) {
         Some(plugin) => plugin,
         None => {
            self.plugins.insert(P::typename(), Arc::new(Mutex::new(P::new())));
            Arc::clone(self.plugins.get(P::typename().as_str()).unwrap())
         }
      }
   }

   pub fn find_plugin(&mut self, name: String) -> Option<PluginHandle> {
      match self.plugins.get(name.as_str()) {
         Some(plugin) => Some(Arc::clone(plugin)),
         None => None
      }
   }

   pub fn get_channel(&mut self, name: String) -> ChannelHandle {
      match self.channels.get(name.as_str()) {
         Some(channel) => Arc::clone(channel),
         None => {
            let (tx, _) = channel(32);
            let _name = name.clone();
            self.channels.insert(_name, Arc::new(Mutex::new(tx)));
            Arc::clone(self.channels.get(name.as_str()).unwrap())
         }
      }
   }

   pub fn subscribe_channel(&mut self, name: String) -> SubscribeHandle {
      Arc::new(FutureMutex::new(self.get_channel(name).lock().unwrap().subscribe()))
   }
}