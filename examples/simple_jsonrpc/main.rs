mod plugin;

use appbase::APP;
use plugin::*;

#[tokio::main]
async fn main() {
   unsafe {
      APP.register_plugin::<monitor::MonitorPlugin>();
      APP.initialize();
      APP.startup();
      APP.execute().await; // XXX: a better way for graceful shutdown?
   }
}
