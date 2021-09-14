use appbase::prelude::*;

use crate::plugin::heartbeat::HeartbeatPlugin;
use crate::plugin::jsonrpc::JsonRpcPlugin;

#[appbase_plugin(HeartbeatPlugin, JsonRpcPlugin)]
pub struct MonitorPlugin {}

impl Plugin for MonitorPlugin {
   fn new() -> Self { Self {} }

   fn startup(&mut self) {
      let mut monitor = APP.channels.subscribe("message");
      let app = APP.quit_handle().unwrap();
      std::thread::spawn(move || {
         loop {
            if app.is_quitting() {
               break;
            }
            if let Ok(message) = monitor.try_recv() {
               println!("{}", message);
            }
         }
      });
   }
}
