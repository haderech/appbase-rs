mod plugin;

use appbase::app;
use plugin::*;
use env_logger;

#[tokio::main]
async fn main() {
   env_logger::init();
   app::register_plugin::<monitor::MonitorPlugin>();
   app::initialize();
   app::startup();
   app::execute().await; // XXX: a better way for graceful shutdown?
}
