extern crate mopa;

use mopa::mopafy;

pub trait Plugin: mopa::Any + Sync + Send {
   fn new() -> Self where Self: Sized;
   fn typename() -> String where Self: Sized;
   fn name(&self) -> String;
   fn initialize(&mut self);
   fn startup(&mut self);
   fn shutdown(&mut self);
   fn state(&self) -> PluginState;
}
mopafy!(Plugin);

#[derive(PartialEq, Copy, Clone)]
pub enum PluginState {
   Registered,
   Initialized,
   Started,
   Stopped,
}

pub struct PluginBase {
   pub state: PluginState,
}

impl PluginBase {
   pub fn new() -> Self {
      PluginBase {
         state: PluginState::Registered,
      }
   }
}

// XXX: substitute Result<(),Error> for simple bool return type
pub trait PluginDeps {
   fn plugin_initialize(&mut self) -> bool;
   fn plugin_startup(&mut self) -> bool;
   fn plugin_shutdown(&mut self) -> bool;
}

#[macro_export]
macro_rules! appbase_plugin_default {
   ($name:ty) => {
      fn typename() -> String {
         stringify!($name).to_string()
      }
      fn name(&self) -> String {
         <$name>::typename()
      }
      fn state(&self) -> PluginState {
         self.base.state
      }
   };
}

#[macro_export]
macro_rules! appbase_plugin_requires_visit {
   ($name:ty, $method:ident) => {
      let mut _p1 = app::get_plugin::<$name>();
      if let Ok(mut plugin) = _p1.try_lock() {
        plugin.$method();
      }
   };
}

#[macro_export]
macro_rules! appbase_plugin_requires {
   ($name:ty; $($deps:ty),*) => {
      impl PluginDeps for $name {
         fn plugin_initialize(&mut self) -> bool {
            if self.base.state != PluginState::Registered {
               return false;
            }
            self.base.state = PluginState::Initialized;
            $(appbase_plugin_requires_visit!($deps, initialize);)*
            log::info!("plugin initialized: {}", <$name>::typename());
            true
         }

         fn plugin_startup(&mut self) -> bool {
            if self.base.state != PluginState::Initialized {
               return false;
            }
            self.base.state = PluginState::Started;
            $(appbase_plugin_requires_visit!($deps, startup);)*
            app::plugin_started::<$name>();
            log::info!("plugin started: {}", <$name>::typename());
            true
         }

         fn plugin_shutdown(&mut self) -> bool {
            if self.base.state != PluginState::Started {
               return false;
            }
            self.base.state = PluginState::Stopped;
            log::info!("plugin shutdown: {}", <$name>::typename());
            true
         }
      }
   };
}
