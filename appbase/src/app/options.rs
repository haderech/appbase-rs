use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

pub struct Options {
   app: Mutex<Option<clap::App<'static>>>,
   parsed: RwLock<Option<clap::ArgMatches>>,
   toml: RwLock<Option<toml::Value>>,
}

impl Options {
   pub fn new(s: &str) -> Self {
      Options {
         app: Mutex::new(Some(clap::App::new(s)
            .arg(clap::Arg::new("config-dir").long("config-dir").takes_value(true))
            .arg(clap::Arg::new("app::plugin").long("plugin").takes_value(true).multiple_occurrences(true)))),
         parsed: RwLock::new(None),
         toml: RwLock::new(None),
      }
   }

   pub fn name(&self, s: &str) {
      let inner: clap::App<'static>;
      {
         let app = self.app.try_lock();
         if app.is_err() {
            panic!("locked: app options");
         }
         inner = app.unwrap().take().unwrap().name(s);
      }
      self.app.try_lock().unwrap().replace(inner);
   }

   pub fn is_parsed(&self) -> bool {
      self.app.try_lock().unwrap().is_none()
   }

   pub fn parse(&self) {
      {
         let app = self.app.try_lock();
         if app.is_err() {
            panic!("locked: app options");
         }

         // parse CLI options
         let parsed = self.parsed.try_write();
         if parsed.is_err() {
            panic!("locked: parsed options");
         }
         parsed.unwrap().replace(app.unwrap().take().unwrap().get_matches());
      } // drop `app`, `parsed` lock

      // parse config.toml file (if exists)
      let mut pathbuf: PathBuf;
      if let Some(config_dir) = self.value_of("config-dir") {
         pathbuf = Path::new(&config_dir).join(Path::new("config.toml"));
      } else {
         pathbuf = directories::BaseDirs::new().unwrap().config_dir().join(Path::new(&crate::util::current_exe()));
         pathbuf.push(Path::new("config/config.toml"));
      }
      if pathbuf.as_path().exists() {
         let mut file = std::fs::File::open(pathbuf.as_path()).unwrap();
         let mut data = String::new();
         let _ = file.read_to_string(&mut data);
         let toml = self.toml.try_write();
         if toml.is_err() {
            panic!("locked: toml options");
         }
         toml.unwrap().replace(data.parse::<toml::Value>().unwrap());
      }
   }

   pub fn arg<A: Into<clap::Arg<'static>>>(&self, a: A) {
      let inner: clap::App<'static>;
      {
         let app = self.app.try_lock();
         if app.is_err() {
            panic!("locked: app options");
         }
         inner = app.unwrap().take().unwrap().arg(a);
      }
      self.app.try_lock().unwrap().replace(inner);
   }

   pub fn is_present(&self, id: &str) -> bool {
      {
         let parsed = self.parsed.try_read().unwrap();
         if parsed.as_ref().unwrap().is_present(id) {
            return true;
         }
      }

      if let Some(value) = self.value_from_toml(id) {
         return value.as_bool().unwrap();
      }
      false
   }

   pub fn value_of(&self, id: &str) -> Option<String> {
      {
         let parsed = self.parsed.try_read().unwrap();
         if let Some(value) = parsed.as_ref().unwrap().value_of(id) {
            return Some(String::from(value));
         }
      }

      if let Some(value) = self.value_from_toml(id) {
         if let Some(s) = value.as_str() {
            return Some(String::from(s));
         }
      }
      None
   }

   pub fn value_of_t<R>(&self, id: &str) -> Option<R> where R: std::str::FromStr + serde::Deserialize<'static>, <R as std::str::FromStr>::Err: std::fmt::Display {
      {
         let parsed = self.parsed.try_read().unwrap();
         if let Ok(value) = parsed.as_ref().unwrap().value_of_t::<R>(id) {
            return Some(value);
         }
      }

      if let Some(value) = self.value_from_toml(id) {
         return value.clone().try_into::<R>().ok();
      }
      None
   }

   pub fn values_of(&self, id: &str) -> Option<Vec<String>> {
      {
         let parsed = self.parsed.try_read().unwrap();
         if let Some(value) = parsed.as_ref().unwrap().values_of(id) {
            return Some(value.map(|v| String::from(v)).collect());
         }
      }

      if let Some(value) = self.value_from_toml(id) {
         return Some(value.as_array().unwrap().iter().map(|x| String::from(x.as_str().unwrap())).collect());
      }
      None
   }

   pub fn values_of_t<R>(&self, id: &str) -> Option<Vec<R>> where R: std::str::FromStr + serde::Deserialize<'static>, <R as std::str::FromStr>::Err: std::fmt::Display {
      {
         let parsed = self.parsed.try_read().unwrap();
         if let Ok(value) = parsed.as_ref().unwrap().values_of_t::<R>(id) {
            return Some(value);
         }
      }

      if let Some(value) = self.value_from_toml(id) {
         return Some(value.as_array().unwrap().iter().map(|x| x.clone().try_into::<R>().unwrap()).collect());
      }
      None
   }

   fn value_from_toml(&self, id: &str) -> Option<toml::Value> {
      let toml= self.toml.try_read().unwrap();
      if toml.is_some() {
         let mut ids = id.split("::");
         if let Some(group) = toml.as_ref().unwrap().as_table().unwrap().get(ids.next().unwrap()) {
            if let Some(value) = group.as_table().unwrap().get(ids.next().unwrap()) {
               return Some(value.clone());
            }
         }
      }
      None
   }
}
