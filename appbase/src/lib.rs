mod util;

pub mod app;
pub mod channel;
pub mod plugin;

pub mod prelude {
   pub use crate::app::{APP, App, QuitHandle};
   pub use crate::plugin::{Base, Plugin};
   pub use crate::channel::{Sender, Receiver};
   pub use appbase_macros::appbase_plugin;
}
