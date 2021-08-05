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
      if let Some(monitor) = &self.monitor {
         MonitorPlugin::watch(monitor.clone());
      }
   }

   fn shutdown(&mut self) {
   }
}

impl MonitorPlugin {
   fn watch(monitor: SubscribeHandle) {
      tokio::spawn(async move {
         if let Ok(message) = monitor.lock().await.try_recv() {
            println!("{}", message);
         }
         if !app::is_quiting() {
            MonitorPlugin::watch(monitor.clone());
         }
      });
   }
}
