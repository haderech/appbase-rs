mod util;

pub mod app;
pub mod channel;
pub mod plugin;

pub mod prelude {
	pub use crate::{
		app::{App, QuitHandle, APP},
		channel::{Receiver, Sender},
		plugin::{Base, Plugin},
	};
	pub use appbase_macros::appbase_plugin;
}
