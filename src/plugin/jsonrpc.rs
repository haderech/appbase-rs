use crate::{appbase_plugin_default, appbase_plugin_requires};
use crate::plugin::{Plugin, PluginBase, PluginState, PluginDeps};
use crate::application::APP;
use jsonrpc_core::{IoHandler, RpcMethodSync, RpcMethodSimple};
use jsonrpc_http_server::{ServerBuilder, CloseHandle};
use std::net::{SocketAddr, IpAddr, Ipv4Addr};

pub struct JsonRpcPlugin {
   base: PluginBase,
   io: Option<IoHandler>,
   server: Option<CloseHandle>,
}

appbase_plugin_requires!(JsonRpcPlugin; );

/*
 * `add_sync_method` and `add_method` SHOULD be called during plugin initialization.
 * After JsonRpcPlugin starts, IoHandler moves into closure, so not available to access from plugin.
 */
impl JsonRpcPlugin {
   pub fn add_sync_method<F>(&mut self, name: String, func: F) where F: RpcMethodSync {
      match self.io.as_mut() {
         Some(io) => io.add_sync_method(name.as_str(), func),
         None => log::error!("add method not available")
      }
   }

   pub fn add_method<F>(&mut self, name: String, func: F) where F: RpcMethodSimple {
      match self.io.as_mut() {
         Some(io) => io.add_method(name.as_str(), func),
         None => log::error!("add method not available")
      }
   }
}

impl Plugin for JsonRpcPlugin {
   appbase_plugin_default!(JsonRpcPlugin);

   fn new() -> Self {
      JsonRpcPlugin {
         base: PluginBase::new(),
         io: None,
         server: None,
      }
   }

   fn initialize(&mut self) {
      if !self.plugin_initialize() {
         return;
      }

      self.io = Some(IoHandler::new());
   }

   fn startup(&mut self) {
      if !self.plugin_startup() {
         return;
      }

      let io = std::mem::replace(&mut self.io, None).unwrap();
      let _server = ServerBuilder::new(io);
      let socket = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
      let server = _server.start_http(&socket).unwrap();
      self.server = Some(server.close_handle());
      tokio::spawn(async {
         server.wait();
      });

      unsafe {
         APP.plugin_started::<JsonRpcPlugin>();
      }
   }

   fn shutdown(&mut self) {
      if !self.plugin_shutdown() {
         return;
      }

      if self.server.is_some() {
         let server = std::mem::replace(&mut self.server, None).unwrap();
         server.close();
      }
   }
}