use appbase::prelude::*;

use crate::plugin::heartbeat::HeartbeatPlugin;
use crate::plugin::jsonrpc::JsonRpcPlugin;

#[appbase_plugin(HeartbeatPlugin, JsonRpcPlugin)]
pub struct MonitorPlugin {
   monitor: Option<Receiver>,
}

impl Plugin for MonitorPlugin {
   fn new() -> Self {
      MonitorPlugin {
         monitor: None,
      }
   }

   fn init(&mut self) {
      self.monitor.replace(APP.channels.subscribe("message"));
   }

   fn startup(&mut self) {
      let mut monitor = self.monitor.take().unwrap();
      let app = APP.quit_handle().unwrap();
      std::thread::spawn(move || {
         loop {
            if app.is_quiting() {
               break;
            }
            if let Ok(message) = monitor.try_recv() {
               println!("{}", message);
            }
         }
      });
   }
}
