use std::sync::Arc;

use jsonrpc_core::{Params, Value};
use tokio::time::{Duration, sleep};

use appbase::*;

use crate::jsonrpc::JsonRpcPlugin;

pub struct HeartbeatPlugin {
    base: PluginBase,
    channel: Option<ChannelHandle>,
}

appbase_plugin_requires!(HeartbeatPlugin; JsonRpcPlugin);

impl Plugin for HeartbeatPlugin {
    appbase_plugin_default!(HeartbeatPlugin);

    fn new() -> Self {
        HeartbeatPlugin {
            base: PluginBase::new(),
            channel: None,
        }
    }

    fn initialize(&mut self) {
        if !self.plugin_initialize() {
            return;
        }

        unsafe {
            self.channel = Some(APP.get_channel("message".to_string()));
        }
        let channel = Arc::clone(&self.channel.as_ref().unwrap());

        let mut _p1: PluginHandle;
        unsafe {
            _p1 = APP.get_plugin::<JsonRpcPlugin>();
        }
        let mut plugin = _p1.lock().unwrap();
        let jsonrpc = plugin.downcast_mut::<JsonRpcPlugin>().unwrap();
        jsonrpc.add_sync_method("bounce".to_string(), move |_: Params| {
            channel
                .lock()
                .unwrap()
                .send(Value::String("Bounce!".to_string()))
                .unwrap();
            Ok(Value::String("Bounce!".to_string()))
        });
    }

    fn startup(&mut self) {
        if !self.plugin_startup() {
            return;
        }

        let channel = Arc::clone(&self.channel.as_ref().unwrap());
        tokio::spawn(async move {
            loop {
                channel
                    .lock()
                    .unwrap()
                    .send(Value::String("Alive!".to_string()))
                    .unwrap();
                sleep(Duration::from_secs(1)).await;
            }
        });
    }

    fn shutdown(&mut self) {
        if !self.plugin_shutdown() {
            return;
        }
    }
}
