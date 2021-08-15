use jsonrpc_core::{Params, Value};
use tokio::time::{Duration, sleep};

use appbase::prelude::*;

use crate::plugin::jsonrpc::JsonRpcPlugin;

#[appbase_plugin(JsonRpcPlugin)]
pub struct HeartbeatPlugin {
   channel: Option<Sender>,
}

impl Plugin for HeartbeatPlugin {
   fn new() -> Self {
      HeartbeatPlugin {
         channel: None,
      }
   }

   fn init(&mut self) {
      let channel = APP.channels.get("message");
      self.channel.replace(channel.clone());

      APP.run_with(|jsonrpc: &mut JsonRpcPlugin| {
         jsonrpc.add_sync_method("bounce".to_string(), move |_: Params| {
            channel.send(Value::String("Bounce!".to_string())).unwrap();
            Ok(Value::String("Bounce!".to_string()))
         });
      });
   }

   fn startup(&mut self) {
      let channel = self.channel.take().unwrap();
      let app = APP.quit_handle().unwrap();
      Self::pulse(channel, app);
   }
}

impl HeartbeatPlugin {
   fn pulse(channel: Sender, app: QuitHandle) {
      APP.spawn(async move {
         channel.send(Value::String("Alive!".to_string())).unwrap();
         sleep(Duration::from_secs(1)).await;
         if !app.is_quiting() {
            Self::pulse(channel, app);
         }
      });
   }
}
