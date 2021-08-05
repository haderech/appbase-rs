extern crate mopa;

use async_trait::async_trait;
use mopa::mopafy;
use tokio::task::JoinHandle;

pub trait Plugin: mopa::Any + Sync + Send + PluginDeps {
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
   pub handle: Option<JoinHandle<()>>,
}

impl PluginBase {
   pub fn new() -> Self {
      PluginBase {
         state: PluginState::Registered,
         handle: None,
      }
   }
}

#[async_trait]
pub trait PluginDeps {
   fn plugin_initialize(&mut self);
   fn plugin_startup(&mut self);
   async fn plugin_shutdown(&mut self);
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
      #[::async_trait::async_trait]
      impl PluginDeps for $name {
         fn plugin_initialize(&mut self) {
            if self.base.state != PluginState::Registered {
               return
            }
            self.base.state = PluginState::Initialized;
            $(appbase_plugin_requires_visit!($deps, plugin_initialize);)*
            self.initialize();
            log::info!("plugin initialized: {}", <$name>::typename());
         }

         fn plugin_startup(&mut self) {
            if self.base.state != PluginState::Initialized {
               return
            }
            self.base.state = PluginState::Started;
            $(appbase_plugin_requires_visit!($deps, plugin_startup);)*
            self.startup();
            app::plugin_started::<$name>();
            log::info!("plugin started: {}", <$name>::typename());
         }

         async fn plugin_shutdown(&mut self) {
            if self.base.state != PluginState::Started {
               return
            }
            self.base.state = PluginState::Stopped;
            if let Some(handle) = self.base.handle.take() {
               let _ = handle.await;
            }
            self.shutdown();
            log::info!("plugin shutdown: {}", <$name>::typename());
         }
      }
   };
}

#[macro_export]
macro_rules! appbase_register_async_loop {
   ($self:ident, $run:block) => {
      $self.base.handle = Some(tokio::task::spawn_blocking( move || {
         loop {
            if app::is_quiting() {
               break;
            }
            $run;
         }
      }));
   }
}
