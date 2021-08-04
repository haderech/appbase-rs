use std::sync::Arc;

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
      if !self.plugin_initialize() {
         return;
      }

      self.monitor = Some(app::subscribe_channel("message".to_string()));
   }

   fn startup(&mut self) {
      if !self.plugin_startup() {
         return;
      }

      let monitor = Arc::clone(self.monitor.as_ref().unwrap());
      appbase_register_async_loop!(
         self,
         {
            if let Ok(message) = monitor.lock().await.try_recv() {
               println!("{}", message);
            }
         }
      );
   }

   fn shutdown(&mut self) {
      if !self.plugin_shutdown() {
         return;
      }
   }
}
