mod plugin;

use appbase::app;
use env_logger;
use plugin::monitor::MonitorPlugin;

fn main() {
   env_logger::init();
   app::register_plugin::<MonitorPlugin>();
   //app::initialize!(MonitorPlugin);
   app::initialize!();
   app::startup();
   app::execute();
}
