use mopa::mopafy;

pub trait Plugin: mopa::Any + Sync + Send + ResolveDeps {
   fn new() -> Self where Self: Sized;
   fn initialize(&mut self);
   fn startup(&mut self);
   fn shutdown(&mut self);
}
mopafy!(Plugin);

#[derive(PartialEq, Copy, Clone)]
pub enum State {
   Registered,
   Initialized,
   Started,
   Stopped,
}

pub trait ResolveDeps {
   fn type_name() -> &'static str where Self: Sized;
   fn resolve_deps(&mut self);
   fn plugin_initialize(&mut self);
   fn plugin_startup(&mut self);
   fn plugin_shutdown(&mut self);
}

#[macro_export]
macro_rules! requires_visit {
   ($name:ty, $method:ident) => {
      if let Ok(mut plugin) = ::appbase::app::get_plugin::<$name>().try_lock() {
         plugin.$method();
      }
   };
}
pub use requires_visit;

#[macro_export]
macro_rules! requires {
   ($name:ty; $($deps:ty),*) => {
      impl ::appbase::plugin::ResolveDeps for $name {
         fn type_name() -> &'static str {
            stringify!($name)
         }

         fn resolve_deps(&mut self) {
            $(::appbase::app::register_plugin::<$deps>();)*
         }

         fn plugin_initialize(&mut self) {
            if ::appbase::app::plugin_initialized::<$name>() {
               return;
            }
            $(::appbase::plugin::requires_visit!($deps, plugin_initialize);)*
            self.initialize();
            log::info!("plugin_initialize");
         }

         fn plugin_startup(&mut self) {
            if ::appbase::app::plugin_started::<$name>() {
               return;
            }
            $(::appbase::plugin::requires_visit!($deps, plugin_startup);)*
            self.startup();
            log::info!("plugin_startup");
         }

         fn plugin_shutdown(&mut self) {
            self.shutdown();
            log::info!("plugin_shutdown");
         }
      }
   };
}
pub use requires;
