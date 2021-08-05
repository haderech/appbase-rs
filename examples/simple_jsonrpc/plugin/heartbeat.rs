use jsonrpc_core::{Params, Value};
use tokio::time::{Duration, sleep};

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
      HeartbeatPlugin::pulse(self.channel.clone().unwrap());
   }

   fn shutdown(&mut self) {
   }
}

impl HeartbeatPlugin {
   fn pulse(channel: ChannelHandle) {
      tokio::spawn(async {
         channel.lock().unwrap().send(Value::String("Alive!".to_string())).unwrap();
         sleep(Duration::from_secs(1)).await;
         if !app::is_quiting() {
            HeartbeatPlugin::pulse(channel);
         }
      });
   }
}
