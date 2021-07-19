mod plugin;

use appbase::app;
use plugin::*;

#[tokio::main]
async fn main() {
   app::register_plugin::<monitor::MonitorPlugin>();
   app::initialize();
   app::startup();
   app::execute().await; // XXX: a better way for graceful shutdown?
}
