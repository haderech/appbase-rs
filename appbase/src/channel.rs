use tokio::sync::broadcast;
use serde_json::Value;

pub type Sender = broadcast::Sender<Value>;
pub type Receiver = broadcast::Receiver<Value>;
