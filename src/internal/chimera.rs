use std::sync::Arc;
use tokio::sync::RwLock;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub path: String,
    pub port: u16,
    pub mode: String,
    #[serde(skip)]
    pub json_value: Arc<RwLock<Value>>,
    pub latency: u64,
    pub sort_rules: HashMap<String, (String, String)>,
    pub paginate: u64,
}

pub struct AppState {
    pub path: String,
    pub port: u16,
    pub json_value: Arc<RwLock<Value>>,
    pub latency: u64,
    pub sort_rules: HashMap<String, (String, String)>,
    pub paginate: u64,
}