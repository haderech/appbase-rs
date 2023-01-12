use std::collections::HashMap;
use std::sync::RwLock;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde_json::Value;
use tokio::sync::broadcast;

pub type Sender = broadcast::Sender<Value>;
pub type Receiver = broadcast::Receiver<Value>;

pub struct Channels {
   map: RwLock<HashMap<String, Sender>>,
   capacity: AtomicUsize,
}

impl Channels {
   pub(super) fn new() -> Self {
      Channels {
         map: RwLock::new(HashMap::new()),
         capacity: AtomicUsize::new(32),
      }
   }

   pub fn get(&self, ch: &str) -> Sender {
      match self.map.try_read().unwrap().get(ch).map(|c| c.clone()) {
         Some(c) => return c,
         None => (),
      }
      self.map.try_write().map(|mut m| {
         let (tx, _) = broadcast::channel(self.capacity.load(Ordering::Acquire));
         m.insert(String::from(ch), tx.clone());
         tx
      }).unwrap()
   }

   pub fn subscribe(&self, ch: &str) -> Receiver {
      self.get(ch).subscribe()
   }

   pub fn set_capacity(&self, capacity: usize) {
      self.capacity.store(capacity, Ordering::Release);
   }
}
