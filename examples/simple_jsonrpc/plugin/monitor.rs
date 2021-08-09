use appbase::*;

use crate::plugin::heartbeat::HeartbeatPlugin;
use crate::plugin::jsonrpc::JsonRpcPlugin;

pub struct MonitorPlugin {
   monitor: Option<channel::Receiver>,
}

plugin::requires!(MonitorPlugin; HeartbeatPlugin, JsonRpcPlugin);

impl Plugin for MonitorPlugin {
   fn new() -> Self {
      MonitorPlugin {
         monitor: None,
      }
   }

   fn initialize(&mut self) {
      self.monitor.replace(app::subscribe_channel("message".to_string()));
   }

   fn startup(&mut self) {
      let mut monitor = self.monitor.take().unwrap();
      let app = app::quit_handle().unwrap();
      app::spawn_blocking(move || {
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

   fn shutdown(&mut self) {
   }
}
