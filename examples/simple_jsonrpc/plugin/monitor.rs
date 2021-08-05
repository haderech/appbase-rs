use appbase::*;

use crate::heartbeat::HeartbeatPlugin;
use crate::jsonrpc::JsonRpcPlugin;

pub struct MonitorPlugin {
   base: PluginBase,
   monitor: Option<SubscribeHandle>,
}

appbase_plugin_requires!(MonitorPlugin; HeartbeatPlugin, JsonRpcPlugin);

impl Plugin for MonitorPlugin {
   appbase_plugin_default!(MonitorPlugin);

   fn new() -> Self {
      MonitorPlugin {
         base: PluginBase::new(),
         monitor: None,
      }
   }

   fn initialize(&mut self) {
      self.monitor.replace(app::subscribe_channel("message".to_string()));
   }

   fn startup(&mut self) {
      let monitor = self.monitor.as_ref().unwrap().clone();
      appbase_register_async_loop!(
         self,
         {
            if let Ok(message) = monitor.try_lock().unwrap().try_recv() {
               println!("{}", message);
            }
         }
      );
   }

   fn shutdown(&mut self) {
   }
}
