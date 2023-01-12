use serde_json::Value;
use tokio::sync::broadcast;

pub type Sender = broadcast::Sender<Value>;
pub type Receiver = broadcast::Receiver<Value>;
