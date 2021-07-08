#[macro_use]
pub mod application;
pub mod plugin;

use application::APP;
// MonitorPlugin depends on HeartbeatPlugin and JsonRpcPlugin.
// Dependencies are automatically loaded.
// use plugin::heartbeat::HeartbeatPlugin;
// use plugin::jsonrpc::JsonRpcPlugin;
use plugin::monitor::MonitorPlugin;

#[tokio::main]
async fn main() {
   env_logger::init();
   unsafe {
      APP.register_plugin::<MonitorPlugin>();
      APP.initialize();
      APP.startup();
      APP.execute().await; // XXX: a better way for graceful shutdown?
   }
}
