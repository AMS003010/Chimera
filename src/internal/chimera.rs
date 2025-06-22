use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;

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
    pub max_request_path_id_length: usize,
    pub max_request_path_len: usize,
    pub cors_enabled: bool,
    pub allowed_origins: Vec<String>,
}

pub struct AppState {
    pub json_value: Arc<RwLock<Value>>,
    pub latency: u64,
    pub sort_rules: HashMap<String, (String, String)>,
    pub paginate: u64,
    pub max_request_path_id_length: usize,
    pub max_request_path_len: usize,
}

pub const CHIMERA_LATEST_VERSION: &str = "0.5.0";

// Change VERSION in https://img.shields.io/badge/version-0.5.0-blue.svg
// Change VERSION in Invoke-WebRequest -Uri "https://github.com/AMS003010/Chimera/releases/download/v0.5.0/chimera-windows.exe" -OutFile "chimera.exe"
// Change VERSION in docs (website)
// Change VERSION in Cargo.toml
