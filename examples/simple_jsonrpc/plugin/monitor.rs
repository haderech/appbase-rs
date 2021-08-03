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
      MonitorPlugin::watch(monitor);
   }

   fn shutdown(&mut self) {
      if !self.plugin_shutdown() {
         return;
      }
   }
}

impl MonitorPlugin {
   fn watch(monitor: SubscribeHandle) {
      tokio::spawn(async move {
         let mut m1 = monitor.lock().await;
         if let Ok(message) = m1.try_recv() {
            println!("{}", message);
         }
         if !app::is_quiting() {
            MonitorPlugin::watch(monitor.clone());
         }
      });
   }
}
