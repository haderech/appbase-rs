use jsonrpc_core::{Params, Value};
use std::thread::sleep;
use std::time::Duration;

use appbase::*;

use crate::jsonrpc::JsonRpcPlugin;

pub struct HeartbeatPlugin {
   base: PluginBase,
   channel: Option<ChannelHandle>,
}

appbase_plugin_requires!(HeartbeatPlugin; JsonRpcPlugin);

impl Plugin for HeartbeatPlugin {
   appbase_plugin_default!(HeartbeatPlugin);

   fn new() -> Self {
      HeartbeatPlugin {
         base: PluginBase::new(),
         channel: None,
      }
   }

   fn initialize(&mut self) {
      let channel = app::get_channel("message".to_string());
      self.channel.replace(channel.clone());

      if let Ok(mut plugin) = app::get_plugin::<JsonRpcPlugin>().lock() {
         let jsonrpc = plugin.downcast_mut::<JsonRpcPlugin>().unwrap();
         jsonrpc.add_sync_method("bounce".to_string(), move |_: Params| {
            channel.lock().unwrap().send(Value::String("Bounce!".to_string())).unwrap();
            Ok(Value::String("Bounce!".to_string()))
         });
      }
   }

   fn startup(&mut self) {
      let channel = self.channel.as_ref().unwrap().clone();
      appbase_register_async_loop!(
         self,
         {
            channel.lock().unwrap().send(Value::String("Alive!".to_string())).unwrap();
            sleep(Duration::from_secs(1));
         }
      );
   }

   fn shutdown(&mut self) {
   }
}
