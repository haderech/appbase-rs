use jsonrpc_core::{Params, Value};
use tokio::time::{Duration, sleep};

use appbase::*;

use crate::jsonrpc::JsonRpcPlugin;

pub struct HeartbeatPlugin {
   channel: Option<ChannelHandle>,
}

appbase_plugin_requires!(HeartbeatPlugin; JsonRpcPlugin);

impl Plugin for HeartbeatPlugin {
   fn new() -> Self {
      HeartbeatPlugin {
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
      let app = app::quit_handle().unwrap();
      HeartbeatPlugin::pulse(channel, app);
   }

   fn shutdown(&mut self) {
   }
}

impl HeartbeatPlugin {
   fn pulse(channel: ChannelHandle, app: QuitHandle) {
      tokio::spawn(async move {
         channel.lock().unwrap().send(Value::String("Alive!".to_string())).unwrap();
         sleep(Duration::from_secs(1)).await;
         if !app.is_quiting() {
            HeartbeatPlugin::pulse(channel, app);
         }
      });
   }
}
