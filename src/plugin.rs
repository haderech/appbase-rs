extern crate mopa;

use async_trait::async_trait;
use mopa::mopafy;

pub trait Plugin: mopa::Any + Sync + Send + PluginDeps {
   fn new() -> Self where Self: Sized;
   fn initialize(&mut self);
   fn startup(&mut self);
   fn shutdown(&mut self);
}
mopafy!(Plugin);

#[derive(PartialEq, Copy, Clone)]
pub enum PluginState {
   Registered,
   Initialized,
   Started,
   Stopped,
}

#[async_trait]
pub trait PluginDeps {
   fn type_name() -> &'static str where Self: Sized;
   fn resolve_deps(&mut self);
   fn plugin_initialize(&mut self);
   fn plugin_startup(&mut self);
   async fn plugin_shutdown(&mut self);
}

#[macro_export]
macro_rules! plugin_requires_visit {
   ($name:ty, $method:ident) => {
      if let Ok(mut plugin) = app::get_plugin::<$name>().try_lock() {
        plugin.$method();
      }
   };
}

#[macro_export]
macro_rules! plugin_requires {
   ($name:ty; $($deps:ty),*) => {
      #[::appbase::async_trait]
      impl PluginDeps for $name {
         fn type_name() -> &'static str {
            stringify!($name)
         }

         fn resolve_deps(&mut self) {
            $(app::register_plugin::<$deps>();)*
         }

         fn plugin_initialize(&mut self) {
            if app::plugin_initialized::<$name>() {
               return;
            }
            $(::appbase::plugin_requires_visit!($deps, plugin_initialize);)*
            self.initialize();
            log::info!("plugin_initialize");
         }

         fn plugin_startup(&mut self) {
            if app::plugin_started::<$name>() {
               return;
            }
            $(::appbase::plugin_requires_visit!($deps, plugin_startup);)*
            self.startup();
            log::info!("plugin_startup");
         }

         async fn plugin_shutdown(&mut self) {
            self.shutdown();
            log::info!("plugin_shutdown");
         }
      }
   };
}
