use crate::{appbase_plugin_default, appbase_plugin_requires, appbase_plugin_requires_visit};
use crate::application::{APP, SubscribeHandle};
use crate::plugin::*;
use crate::plugin::heartbeat::HeartbeatPlugin;
use crate::plugin::jsonrpc::JsonRpcPlugin;
use std::sync::Arc;

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

      unsafe {
         self.monitor = Some(APP.subscribe_channel("message".to_string()));
      }
   }

   fn startup(&mut self) {
      if !self.plugin_startup() {
         return;
      }

      let _m1 = Arc::clone(self.monitor.as_ref().unwrap());
      tokio::spawn(async move {
         let mut monitor = _m1.lock().await;
         loop {
            let message = monitor.recv().await.unwrap();
            println!("{}", message);
         }
      });
   }

   fn shutdown(&mut self) {
      if !self.plugin_shutdown() {
         return;
      }
   }
}