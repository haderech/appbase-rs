use std::io::Read;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, RwLock};

pub struct Options<'a> {
   app: Mutex<Option<clap::App<'a>>>,
   parsed: RwLock<Option<clap::ArgMatches>>,
   toml: RwLock<Option<toml::Value>>,
}

impl<'a> Options<'a> {
   pub(super) fn new(s: &str) -> Self {
      Options {
         app: Mutex::new(Some(clap::App::new(s)
            .arg(clap::Arg::new("config-dir").long("config-dir").takes_value(true))
            .arg(clap::Arg::new("app::plugin").long("plugin").takes_value(true).multiple_occurrences(true))
            .arg(clap::Arg::new("app::channel-capacity").long("channel-capacity").takes_value(true))
            .arg(clap::Arg::new("app::worker-threads").long("worker-threads").takes_value(true))
            .arg(clap::Arg::new("app::max-blocking-threads").long("max-blocking-threads").takes_value(true)))),
         parsed: RwLock::new(None),
         toml: RwLock::new(None),
      }
   }

   pub fn name(&self, s: &str) {
      let mut app = self.app.try_lock().expect("locked: app options");
      let new = app.take().unwrap().name(s);
      app.replace(new);
   }

   pub fn is_parsed(&self) -> bool {
      self.app.try_lock().unwrap().is_none()
   }

   pub fn parse(&self) {
      {
         let mut app = self.app.try_lock().expect("locked: app options");
         // parse CLI options
         let mut parsed = self.parsed.try_write().expect("locked: parsed options");
         parsed.replace(app.take().unwrap().get_matches());
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
         let mut toml = self.toml.try_write().expect("locked: toml options");
         toml.replace(data.parse::<toml::Value>().unwrap());
      }
   }

   pub fn arg<A: Into<clap::Arg<'a>>>(&self, a: A) {
      let mut app = self.app.try_lock().expect("locked: app options");
      let new = app.take().unwrap().arg(a);
      app.replace(new);
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

      self.value_from_toml(id).map(|value| value.as_str().map(|s| String::from(s))).flatten()
   }

   pub fn value_of_t<R>(&self, id: &str) -> Option<R> where R: std::str::FromStr + serde::Deserialize<'a>, <R as std::str::FromStr>::Err: std::fmt::Display {
      {
         let parsed = self.parsed.try_read().unwrap();
         if let Ok(value) = parsed.as_ref().unwrap().value_of_t::<R>(id) {
            return Some(value);
         }
      }

      self.value_from_toml(id).map(|value| value.clone().try_into::<R>().ok()).flatten()
   }

   pub fn values_of(&self, id: &str) -> Option<Vec<String>> {
      {
         let parsed = self.parsed.try_read().unwrap();
         if let Some(value) = parsed.as_ref().unwrap().values_of(id) {
            return Some(value.map(|v| String::from(v)).collect());
         }
      }

      self.value_from_toml(id).map(|value| value.as_array().unwrap().iter().map(|x| String::from(x.as_str().unwrap())).collect())
   }

   pub fn values_of_t<R>(&self, id: &str) -> Option<Vec<R>> where R: std::str::FromStr + serde::Deserialize<'a>, <R as std::str::FromStr>::Err: std::fmt::Display {
      {
         let parsed = self.parsed.try_read().unwrap();
         if let Ok(value) = parsed.as_ref().unwrap().values_of_t::<R>(id) {
            return Some(value);
         }
      }

      self.value_from_toml(id).map(|value| value.as_array().unwrap().iter().map(|x| x.clone().try_into::<R>().unwrap()).collect())
   }

   fn value_from_toml(&self, id: &str) -> Option<toml::Value> {
      match self.toml.try_read().unwrap().as_ref() {
         Some(toml) => {
            let mut ids = id.split("::");
            toml.as_table().unwrap().get(ids.next().unwrap()).map(|group| {
               group.as_table().unwrap().get(ids.next().unwrap()).map(|value| value.clone())
            }).flatten()
         },
         None => None,
      }
   }
}
