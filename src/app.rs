use std::collections::HashMap;
use std::env;
use std::future::Future;
use std::io::Read;
use std::str::FromStr;
use std::sync::{Arc, Mutex};
use std::sync::atomic::{AtomicBool, Ordering};

use once_cell::sync::Lazy;
use serde_json::value::Value;
use tokio::runtime::{Builder, Runtime};
use tokio::sync::{broadcast, watch};
use tokio::task::JoinHandle;

use crate::plugin;
use crate::plugin::Plugin as PluginImpl;

pub mod channel {
   use super::*;
   pub type Sender = broadcast::Sender<Value>;
   pub type Receiver = broadcast::Receiver<Value>;
}

pub type PluginHandle = Arc<Mutex<dyn PluginImpl>>;

pub struct QuitHandle {
   handle: Option<watch::Receiver<bool>>,
}

impl QuitHandle {
   pub fn is_quiting(&self) -> bool {
      let handle = self.handle.as_ref();
      match handle {
         Some(handle) => *handle.borrow(),
         None => true,
      }
   }
   pub fn quit(&mut self) {
      quit();
      self.handle.take();
   }
}

static mut APP: Lazy<Application> = Lazy::new(|| {
   Application::new()
});

struct Application {
   runtime: Arc<Runtime>,
   plugins: HashMap<String, Plugin>,
   running_plugins: Vec<String>,
   channels: HashMap<String, channel::Sender>,
   is_quiting: Arc<AtomicBool>,
   quit_tx: Option<watch::Sender<bool>>,
   quit_rx: Option<watch::Receiver<bool>>,
   options: Option<clap::App<'static>>,
   parsed_options: Option<clap::ArgMatches>,
   toml: Option<toml::Value>,
}

struct Plugin {
   instance: PluginHandle,
   state: plugin::State,
}

impl Plugin {
   fn new<P>() -> Plugin where P: PluginImpl {
      Plugin {
         instance: Arc::new(Mutex::new(P::new())),
         state: plugin::State::Registered,
      }
   }
}

impl Application {
   fn new() -> Application {
      let (tx, rx) = watch::channel(false);
      let mut app = Application {
         runtime: Arc::new(Builder::new_multi_thread().enable_all().build().unwrap()),
         plugins: HashMap::new(),
         running_plugins: Vec::new(),
         channels: HashMap::new(),
         is_quiting: Arc::new(AtomicBool::new(false)),
         quit_tx: Some(tx),
         quit_rx: Some(rx),
         options: None,
         parsed_options: None,
         toml: None,
      };
      let args = env::args().collect::<Vec<String>>();
      let bin = std::path::Path::new(&args[0]).file_stem().unwrap().to_str().unwrap();
      let options = clap::App::new(bin)
         .arg(clap::Arg::new("app::plugin").long("plugin").takes_value(true).multiple_occurrences(true));
      app.options.replace(options);
      app
   }

   fn initialize<P>(&mut self) where P: PluginImpl {
      if let Some(plugin) = self.plugins.get(P::type_name()) {
         if let Ok(mut p1) = plugin.instance.lock() {
            p1.plugin_initialize();
         }
      }
   }

   fn startup(&mut self) {
      if self.is_quiting.load(Ordering::Relaxed) {
         return;
      }
      for plugin in self.plugins.values_mut() {
         if plugin.state != plugin::State::Initialized {
            continue;
         }
         if let Ok(mut p1) = plugin.instance.lock() {
            p1.plugin_startup();
         }
      }
   }

   fn execute(&mut self) {
      self.runtime.block_on(async {
         let mut sigint = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::interrupt()).unwrap();
         let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate()).unwrap();

         let is_quiting = self.is_quiting.clone();
         let quit = tokio::task::spawn_blocking(move || {
            loop {
               if is_quiting.load(Ordering::Relaxed) {
                  break;
               }
            }
         });
         tokio::select! {
            _ = sigint.recv() => {},
            _ = sigterm.recv() => {},
            _ = quit => {},
         }
      });
      self.shutdown();
   }

   fn quit(&mut self) {
      let quit_tx = self.quit_tx.as_ref();
      match quit_tx {
         Some(quit_tx) => {
            let _ = quit_tx.send(true);
            self.quit_rx.take();
            self.is_quiting.store(true, Ordering::Relaxed);
         },
         None => {
            log::warn!("app is already quiting");
         },
      }
   }

   fn shutdown(&mut self) {
      self.quit();
      let quit_tx = self.quit_tx.take().unwrap();
      self.runtime.clone().block_on(async {
         log::info!("try graceful shutdown...");
         // wait until all `quit_handle`s are dropped
         quit_tx.closed().await;
         for typeid in self.running_plugins.iter().rev() {
            let plugin = self.plugins.get_mut(typeid).unwrap();
            if let Ok(mut p1) = plugin.instance.lock() {
               if plugin.state != plugin::State::Started {
                  continue;
               }
               plugin.state = plugin::State::Stopped;
               p1.plugin_shutdown();
            }
         }
      });
   }
}

pub fn initialize_plugin<P>() where P: PluginImpl {
   unsafe {
      APP.initialize::<P>();
   }
}

#[macro_export]
macro_rules! initialize {
   ($($plugin:ty),*) => {
      ::appbase::app::load_toml();
      $(::appbase::app::initialize_plugin::<$plugin>();)*
      if let Some(mut plugins) = ::appbase::app::values_of("app::plugin") {
         let mut iter = plugins.iter();
         while let Some(plugin) = iter.next() {
            ::appbase::app::find_plugin(plugin).unwrap().lock().unwrap().plugin_initialize();
         }
      }
   };
}
pub use initialize;

pub fn startup() {
   unsafe {
      APP.startup();
   }
}

pub fn execute() {
   unsafe {
      APP.execute();
   }
}

pub fn quit() {
   unsafe {
      APP.quit();
   }
}

pub fn plugin_initialized<P>() -> bool where P: PluginImpl {
   unsafe {
      if let Some(plugin) = APP.plugins.get_mut(P::type_name()) {
         if plugin.state == plugin::State::Registered {
            plugin.state = plugin::State::Initialized;
            return false;
         }
      }
      true
   }
}

pub fn plugin_started<P>() -> bool where P: PluginImpl {
   unsafe {
      let type_name = P::type_name();
      if let Some(plugin) = APP.plugins.get_mut(type_name) {
         if plugin.state == plugin::State::Initialized {
            plugin.state = plugin::State::Started;
            APP.running_plugins.push(type_name.to_string());
            return false;
         }
      }
      true
   }
}

pub fn register_plugin<P>() where P: PluginImpl {
   unsafe {
      let type_name = P::type_name();
      if !APP.plugins.contains_key(type_name) {
         APP.plugins.insert(type_name.to_string(), Plugin::new::<P>());
         APP.plugins.get(type_name).unwrap().instance.lock().unwrap().resolve_deps();
      }
   }
}

pub fn get_plugin<P>() -> PluginHandle where P: PluginImpl {
   find_plugin(P::type_name()).unwrap()
}

pub fn find_plugin(type_name: &str) -> Option<PluginHandle> {
   unsafe {
      match APP.plugins.get(type_name) {
         Some(plugin) => Some(plugin.instance.clone()),
         None => None
      }
   }
}

pub fn get_channel(name: String) -> channel::Sender {
   unsafe {
      match APP.channels.get(name.as_str()) {
         Some(channel) => channel.clone(),
         None => {
            let (tx, _) = broadcast::channel(32);
            APP.channels.insert(name.clone(), tx);
            APP.channels.get(name.as_str()).unwrap().clone()
         }
      }
   }
}

pub fn subscribe_channel(name: String) -> channel::Receiver {
   get_channel(name).subscribe()
}

pub fn quit_handle() -> Option<QuitHandle> {
   unsafe {
      if let Some(rx) = APP.quit_rx.as_ref() {
         return Some(QuitHandle{
            handle: Some(rx.clone()),
         });
      }
      None
   }
}

pub fn spawn<F>(future: F) -> JoinHandle<F::Output> where F: Future + Send + 'static, F::Output: Send + 'static {
   unsafe {
      APP.runtime.spawn(future)
   }
}

pub fn spawn_blocking<F, R>(func: F) -> JoinHandle<R> where F: FnOnce() -> R + Send + 'static, R: Send + 'static {
   unsafe {
      APP.runtime.spawn_blocking(func)
   }
}

pub fn plugin_state<P>() -> Option<plugin::State> where P: PluginImpl {
   unsafe {
      if let Some(plugin) = APP.plugins.get(P::type_name()) {
         return Some(plugin.state);
      }
   }
   None
}

pub fn arg(arg: clap::Arg<'static>) {
   unsafe {
      if APP.options.is_some() {
         APP.options = Some(APP.options.take().unwrap().arg(arg));
      } else {
         log::error!("once options being parsed, cannot set more");
      }
   }
}

pub fn is_present(opt: &str) -> bool {
   unsafe {
      if let None = APP.parsed_options {
         APP.parsed_options.replace(APP.options.take().unwrap().get_matches());
      }
      if APP.parsed_options.as_ref().unwrap().is_present(opt) {
         return true;
      }
      if APP.toml.is_some() {
         let mut opts = opt.split("::");
         let namespace = opts.next().unwrap();
         let key = opts.next().unwrap();
         if let Some(table) = APP.toml.as_ref().unwrap().as_table().unwrap().get(namespace) {
            if let Some(item) = table.as_table().unwrap().get(key) {
               return bool::from_str(item.as_str().unwrap()).unwrap();
            }
         }
      }
      false
   }
}

pub fn value_of(opt: &str) -> Option<&'static str> {
   unsafe {
      if let None = APP.parsed_options {
         APP.parsed_options.replace(APP.options.take().unwrap().get_matches());
      }
      if let Some(value) = APP.parsed_options.as_ref().unwrap().value_of(opt) {
         return Some(value);
      }
      if APP.toml.is_some() {
         let mut opts = opt.split("::");
         let namespace = opts.next().unwrap();
         let key = opts.next().unwrap();
         if let Some(table) = APP.toml.as_ref().unwrap().as_table().unwrap().get(namespace) {
            if let Some(item) = table.as_table().unwrap().get(key) {
               return item.as_str();
            }
         }
      }
      None
   }
}

pub fn values_of(opt: &str) -> Option<Vec<&str>> {
   unsafe {
      if let None = APP.parsed_options {
         APP.parsed_options.replace(APP.options.take().unwrap().get_matches());
      }
      if let Some(values) = APP.parsed_options.as_ref().unwrap().values_of(opt) {
         return Some(values.collect());
      }
      if APP.toml.is_some() {
         let mut opts = opt.split("::");
         let namespace = opts.next().unwrap();
         let key = opts.next().unwrap();
         if let Some(table) = APP.toml.as_ref().unwrap().as_table().unwrap().get(namespace) {
            if let Some(item) = table.as_table().unwrap().get(key) {
               return Some(item.as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect());
            }
         }
      }
      None
   }
}

pub fn load_toml() {
   let path = std::path::Path::new("config.toml");
   if path.exists() {
      let mut file = std::fs::File::open(path).unwrap();
      let mut data = String::new();
      let _ = file.read_to_string(&mut data);
      unsafe {
         APP.toml.replace(data.parse::<toml::Value>().unwrap());
      }
   }
}