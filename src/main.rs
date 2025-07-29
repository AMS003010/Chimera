use crate::internal::chimera::{AppState, AppStateWs, Config, CHIMERA_LATEST_VERSION};
use crate::internal::helpers::{find_key_and_id_lengths, shutdown_signal};
use crate::internal::http_handlers::{
    delete_data, get_data, handle_form_submission, patch_data, ping_pong, post_data, put_data,
};
use crate::internal::json_data_generate::{generate_json_from_schema, JsonDataGeneratorSchema};
use crate::internal::port::find_available_port;
use crate::internal::ws_handlers::{handle_websocket, ws_fallback_handler};
use axum::{
    http::Method,
    routing::{delete, get, patch, post, put},
    Router,
};
use clap::{Arg, Command};
use colored::Colorize;
use csv::Reader;
use local_ip_address::local_ip;
use serde_json::{Map, Value};
use std::collections::HashMap;
use std::io::Error as IOError;
use std::path::Path as Std_path;
use std::path::Path;
use std::process;
use std::sync::Arc;
use std::{net::SocketAddr};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};

mod internal {
    pub mod chimera;
    pub mod helpers;
    pub mod http_handlers;
    pub mod json_data_generate;
    pub mod port;
    pub mod ws_handlers;
}

async fn run_axum_server(config: Config) -> Result<(), IOError> {
    let state = Arc::new(AppState {
        json_value: config.json_value,
        latency: config.latency,
        sort_rules: config.sort_rules,
        paginate: config.paginate,
        max_request_path_id_length: config.max_request_path_id_length,
        max_request_path_len: config.max_request_path_len,
        logs_disabled: config.logs_disabled,
    });

    println!("[{}] Running HTTP", "INFO".green());

    let cors_layer = if config.cors_enabled {
        let allowed_origins = config
            .allowed_origins
            .iter()
            .filter_map(|origin| origin.parse::<axum::http::HeaderValue>().ok())
            .collect::<Vec<_>>();

        if config.allowed_origins.is_empty() {
            println!("[{}] CORS: * \n", "INFO".green());
            CorsLayer::new()
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(Any)
                .allow_origin(Any)
                .allow_credentials(false)
        } else {
            println!("[{}] CORS: chimera.cors\n", "INFO".green());
            CorsLayer::new()
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(Any)
                .allow_origin(allowed_origins)
                .allow_credentials(false)
        }
    } else {
        println!("[{}] CORS: * \n", "INFO".green());
        CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers(Any)
            .allow_origin(Any)
    };

    // Build router with Axum
    let app = Router::new()
        .route("/", get(ping_pong))
        .route("/submit-form", post(handle_form_submission))
        .route("/*route", get(get_data))
        .route("/*route", delete(delete_data))
        .route("/*route", post(post_data))
        .route("/*route", put(put_data))
        .route("/*route", patch(patch_data))
        .layer(cors_layer)
        .with_state(state.clone());

    // Address to bind the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    // Display server info
    let local = "127.0.0.1";
    let lan_ip = local_ip().unwrap_or_else(|_| local.parse().unwrap());
    println!(" - Local:     http://{}:{}", local, config.port);
    println!(" - Network:   http://{}:{}\n", lan_ip, config.port);

    // Setup graceful shutdown
    let listener = tokio::net::TcpListener::bind(&addr).await?;

    // Wait for the server to complete (or for a shutdown signal)
    if let Err(e) = axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    {
        eprintln!("Server error: {}", e);
    } else {
        println!("\nReceived shutdown signal, starting graceful shutdown...");
    }

    Ok(())
}

async fn run_grpc_server(config: Config) -> Result<(), IOError> {
    println!("{:#?}", config);

    Ok(())
}

pub async fn run_websocket_server(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let shared_data = config.json_value.clone();
    let state = Arc::new(AppStateWs {
        sort_rules: config.sort_rules,
        paginate: config.paginate,
        logs_disabled: config.logs_disabled,
    });
    let connections = Arc::new(RwLock::new(HashMap::new()));

    println!("[{}] Running Websocket", "INFO".green());

    // Create CORS layer with proper method conversion
    let cors_layer = if config.cors_enabled {
        let allowed_origins = config
            .allowed_origins
            .iter()
            .filter_map(|origin| origin.parse::<axum::http::HeaderValue>().ok())
            .collect::<Vec<_>>();

        if config.allowed_origins.is_empty() {
            println!("[{}] CORS: * \n", "INFO".green());
            CorsLayer::new()
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(Any)
                .allow_origin(Any)
                .allow_credentials(false)
        } else {
            println!("[{}] CORS: chimera.cors\n", "INFO".green());
            CorsLayer::new()
                .allow_methods([
                    Method::GET,
                    Method::POST,
                    Method::PUT,
                    Method::PATCH,
                    Method::DELETE,
                    Method::OPTIONS,
                ])
                .allow_headers(Any)
                .allow_origin(allowed_origins)
                .allow_credentials(false)
        }
    } else {
        println!("[{}] CORS: * \n", "INFO".green());
        CorsLayer::new()
            .allow_methods([
                Method::GET,
                Method::POST,
                Method::PUT,
                Method::PATCH,
                Method::DELETE,
            ])
            .allow_headers(Any)
            .allow_origin(Any)
    };

    let app = Router::new()
        .route("/ws/*route", get(handle_websocket))
        .route("/ws", get(ws_fallback_handler))
        .with_state((state, shared_data, connections))
        .layer(cors_layer);

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    // Display server info
    let local = "127.0.0.1";
    let lan_ip = local_ip().unwrap_or_else(|_| local.parse().unwrap());
    println!(" - Local:     ws://{}:{}/ws", local, config.port);
    println!(" - Network:   ws://{}:{}/ws\n", lan_ip, config.port);

    // Modern Axum server setup
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    axum::serve(
        listener,
        app.into_make_service_with_connect_info::<SocketAddr>(),
    )
    .with_graceful_shutdown(shutdown_signal())
    .await
    .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?;

    Ok(())
}

async fn initialize_cmd() -> Result<Config, IOError> {
    let matches = Command::new("Chimera - Mock SeRVeR")
        .version(CHIMERA_LATEST_VERSION)
        .author("Abhijith M S")
        .about("Multi-Protocol⚡ Mock SeRVeR built in Rust 🦀")
        .arg_required_else_help(true)

        // Common Args
        .arg(Arg::new("path")
            .short('P')
            .long("path")
            .num_args(1)
            .required(true)
            .help("Path to the Json file"))
        .arg(Arg::new("quiet")
            .long("quiet")
            .num_args(0)
            .help("Disable logs"))

        // Args to `http`
        .subcommand(
            Command::new("http")
            .about("Start HTTP REST API server")
            .arg(Arg::new("port")
                .short('p')
                .long("port")
                .num_args(1)
                .default_value("8080")
                .help("Port for the server"))
            .arg(Arg::new("latency")
                .short('L')
                .long("latency")
                .num_args(1)
                .default_value("0")
                .help("Simulate latency (ms) from the server (Throttle)"))
            .arg(Arg::new("sort")
                .short('S')
                .long("sort")
                .num_args(1..)
                .action(clap::ArgAction::Append)
                .help("Sort entries in each route (e.g., --sort <route> <asc|desc> <attribute_in_route>)"))
            .arg(Arg::new("page")
                .short('A')
                .long("page")
                .num_args(1)
                .default_value("0")
                .help("Paginate records in the GET request"))
            .arg(Arg::new("auto_generate_data")
                .short('X')
                .long("auto_generate_data")
                .num_args(0)
                .help("Auto generate data without a .json sample file. A route schema .json file should be passed to --path"))
            .arg(Arg::new("cors")
                .long("cors")
                .num_args(0)
                .help("Enable CORS support (reads allowed domains from chimera.cors file)"))
        )

        // Args to `websocket`
        .subcommand(
            Command::new("websocket")
                .about("Start WebSocket server")
                .arg(Arg::new("port")
                    .short('p')
                    .long("port")
                    .num_args(1)
                    .default_value("8080")
                    .help("Port for the WebSocket server"))
                .arg(Arg::new("sort")
                    .short('S')
                    .long("sort")
                    .num_args(1..)
                    .action(clap::ArgAction::Append)
                    .help("Sort entries in each route (e.g., --sort <route> <asc|desc> <attribute_in_route>)"))
                .arg(Arg::new("page")
                    .short('A')
                    .long("page")
                    .num_args(1)
                    .default_value("0")
                    .help("Paginate records in the GET request"))
                .arg(Arg::new("cors")
                    .long("cors")
                    .num_args(0)
                    .help("Enable CORS support (reads allowed domains from chimera.cors file)"))
                .arg(Arg::new("auto_generate_data")
                    .short('X')
                    .long("auto_generate_data")
                    .num_args(0)
                    .help("Auto generate data without a .json sample file. A route schema .json file should be passed to --path"))
        )
        .get_matches();

    let json_file_path = matches
        .get_one::<String>("path")
        .expect("Missing path argument")
        .to_string();
    let logs_disabled = matches.get_flag("quiet");

    // Default values for subcommand-specific args
    let mut server_port = 8080;
    let mut sim_latency = 0;
    let mut pagination_factor = 0;
    let mut auto_generate_enabled = false;
    let mut cors_enabled = false;
    let mut sort_rules: HashMap<String, (String, String)> = HashMap::new();
    let mut mode = "http";

    if let Some(http_matches) = matches.subcommand_matches("http") {
        server_port = http_matches
            .get_one::<String>("port")
            .unwrap()
            .parse::<u16>()
            .expect("Invalid port number");

        let sim_latency_str = http_matches.get_one::<String>("latency").unwrap();
        sim_latency = sim_latency_str
            .trim_end_matches("ms")
            .parse::<u64>()
            .unwrap_or(0);

        pagination_factor = http_matches
            .get_one::<String>("page")
            .unwrap()
            .parse::<u64>()
            .expect("Invalid page format");

        auto_generate_enabled = http_matches.get_flag("auto_generate_data");
        cors_enabled = http_matches.get_flag("cors");

        if let Some(sort_args) = http_matches.get_many::<String>("sort") {
            let sort_list: Vec<String> = sort_args.map(|s| s.clone()).collect();
            for sort_group in sort_list.chunks(3) {
                if let [route, order, key] = sort_group {
                    sort_rules.insert(route.clone(), (order.clone(), key.clone()));
                }
            }
        }
    }

    if let Some(ws_matches) = matches.subcommand_matches("websocket") {
        mode = "websocket";
        server_port = ws_matches
            .get_one::<String>("port")
            .unwrap()
            .parse::<u16>()
            .expect("Invalid port number");
        cors_enabled = ws_matches.get_flag("cors");
        auto_generate_enabled = ws_matches.get_flag("auto_generate_data");
        pagination_factor = ws_matches
            .get_one::<String>("page")
            .unwrap()
            .parse::<u64>()
            .expect("Invalid page format");
        if let Some(sort_args) = ws_matches.get_many::<String>("sort") {
            let sort_list: Vec<String> = sort_args.map(|s| s.clone()).collect();
            for sort_group in sort_list.chunks(3) {
                if let [route, order, key] = sort_group {
                    sort_rules.insert(route.clone(), (order.clone(), key.clone()));
                }
            }
        }
    }

    let mut allowed_origins = Vec::new();

    if cors_enabled {
        let cors_file = "chimera.cors";
        if Std_path::new(cors_file).exists() {
            allowed_origins = tokio::fs::read_to_string(cors_file)
                .await
                .unwrap_or_default()
                .lines()
                .map(|s| s.trim().to_string())
                .filter(|s| !s.is_empty())
                .collect();
        } else {
            eprintln!(
                "[{}] CORS enabled but chimera.cors file not found. Allowing all origins.",
                "WARN".yellow()
            );
        }
    }

    let json_content = tokio::fs::read_to_string(&json_file_path)
        .await
        .expect("Failed to read file");

    // Check file extension first
    let file_extension = Path::new(&json_file_path)
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("");

    let parsed_content: Value = match file_extension.to_lowercase().as_str() {
        "csv" => {
            let mut reader = Reader::from_reader(json_content.as_bytes());
            let mut schema_entries = Vec::new();

            for result in reader.records() {
                let record = result.expect("Failed to read CSV record");

                if record.len() >= 4 {
                    let path = record.get(0).unwrap_or("").to_string();
                    let no_of_entries: u32 = record
                        .get(1)
                        .unwrap_or("0")
                        .parse()
                        .expect("Invalid no_of_entries value");
                    let null_percentage: u8 = record
                        .get(2)
                        .unwrap_or("0")
                        .parse()
                        .expect("Invalid null_percentage value");
                    let schema_json: Value = serde_json::from_str(record.get(3).unwrap_or("{}"))
                        .expect("Invalid schema JSON in CSV");

                    // Create each route entry
                    let mut route_entry = Map::new();
                    route_entry.insert("path".to_string(), Value::String(path));
                    route_entry.insert(
                        "no_of_entries".to_string(),
                        Value::Number(no_of_entries.into()),
                    );
                    route_entry.insert("schema".to_string(), schema_json);
                    route_entry.insert(
                        "null_percentage".to_string(),
                        Value::Number(null_percentage.into()),
                    );

                    schema_entries.push(Value::Object(route_entry));
                }
            }

            let mut routes_object = Map::new();
            routes_object.insert("routes".to_string(), Value::Array(schema_entries));
            let routes_value = Value::Object(routes_object);

            let schema: JsonDataGeneratorSchema = serde_json::from_value(routes_value)
                .expect("Failed to convert CSV data to schema format");
            generate_json_from_schema(schema)
        }
        "json" => {
            if auto_generate_enabled {
                let schema: JsonDataGeneratorSchema = serde_json::from_str(&json_content)
                    .expect("Invalid schema format for auto data generation");
                generate_json_from_schema(schema)
            } else {
                let content: Value =
                    serde_json::from_str(&json_content).expect("Invalid Json format");

                if let Some(routes) = content.get("routes") {
                    if routes.is_array() {
                        eprintln!("Please pass a data file .json for your routes as `auto-generate-data` is disabled");
                        process::exit(1);
                    } else {
                        eprintln!("Please pass a data file .json for your routes as `auto-generate-data` is disabled");
                        process::exit(1);
                    }
                }

                if !content.is_object() {
                    eprintln!(
                        "Error: The given json file is a JSON Array! It should be a JSON Object"
                    );
                    process::exit(1);
                }
                content
            }
        }
        _ => {
            eprintln!("Error: Unsupported file format. Please provide a .json or .csv file");
            process::exit(1);
        }
    };

    let mut spaces = 0;
    let mut longest_path = 0;

    if let Some((key, len)) = find_key_and_id_lengths(&parsed_content) {
        spaces = len;
        longest_path = key;
    }

    let final_port: u16 = find_available_port(server_port);

    Ok(Config {
        path: json_file_path,
        port: final_port,
        mode: mode.to_string(),
        json_value: Arc::new(RwLock::new(parsed_content)),
        latency: sim_latency,
        sort_rules,
        paginate: pagination_factor,
        max_request_path_id_length: spaces,
        max_request_path_len: longest_path,
        cors_enabled: cors_enabled,
        logs_disabled,
        allowed_origins,
    })
}

#[tokio::main]
async fn main() -> Result<(), IOError> {
    println!(
        "
╔═╗┬ ┬┬┌┬┐┌─┐┬─┐┌─┐
║  ├─┤││││├┤ ├┬┘├─┤
╚═╝┴ ┴┴┴ ┴└─┘┴└─┴ ┴
v{}
    ",
        CHIMERA_LATEST_VERSION
    );

    let config_data = initialize_cmd().await?;

    match config_data.mode.as_str() {
        "http" => {
            if let Err(e) = run_axum_server(config_data).await {
                eprintln!("Failed to run Axum server: {}", e);
            }
        }
        "grpc" => {
            if let Err(e) = run_grpc_server(config_data).await {
                eprintln!("Failed to run Tonic server: {}", e);
            }
        }
        "websocket" => {
            if let Err(e) = run_websocket_server(config_data).await {
                eprintln!("Failed to setup websocket connection: {}", e);
            }
        }
        _ => {
            println!("PROTOCOL NOT SUPPORTED !!");
        }
    }

    println!(
        "
Chimera retreats to the shadows... 
It will rise again. 🐉
    "
    );
    Ok(())
}
