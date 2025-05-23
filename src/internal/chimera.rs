use tokio::sync::Mutex;
use std::collections::HashMap;
use serde::{Serialize, Deserialize};
use serde_json::Value;

#[derive(Serialize, Deserialize, Debug)]
pub struct Config {
    pub path: String,
    pub port: u16,
    #[serde(skip)]
    pub json_value: Mutex<Value>,
    pub latency: u64,
    pub sort_rules: HashMap<String, (String, String)>,
    pub paginate: u64,
}