mod plugin;

use appbase::prelude::*;
use env_logger;

use crate::plugin::MonitorPlugin;

fn main() {
	env_logger::init();
	APP.register::<MonitorPlugin>();
	APP.init();
	APP.startup();
	APP.execute();
}
