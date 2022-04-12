use appbase::prelude::*;
use tokio::time::{Duration, sleep};

use crate::plugin::heartbeat::HeartbeatPlugin;
use crate::plugin::jsonrpc::JsonRpcPlugin;

#[appbase_plugin(HeartbeatPlugin, JsonRpcPlugin)]
pub struct MonitorPlugin {}

impl Plugin for MonitorPlugin {
   fn new() -> Self { Self {} }

   fn startup(&mut self) {
      let monitor = APP.channels.subscribe("message");
      let app = APP.quit_handle().unwrap();
      Self::recv(monitor, app);
   }
}

impl MonitorPlugin {
   fn recv(mut monitor: Receiver,  app: QuitHandle) {
      APP.spawn(async move {
         if let Ok(message) = monitor.try_recv() {
            println!("{}", message);
         }
         sleep(Duration::from_millis(10)).await;
         if !app.is_quitting() {
            Self::recv(monitor, app);
         }
      });
   }
}
