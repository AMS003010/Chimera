use std::io::Error as IOError;
use std::process;
use std::collections::HashMap;
use std::{fs, net::SocketAddr};
use std::sync::Arc;
use axum::{
    extract::{Path, State},
    routing::{get, delete},
    Router, response::{IntoResponse, Response},
    http::{StatusCode, Uri}
};
use hyper::server::Server;
use tokio::time::{sleep, Duration, timeout};
use tokio::sync::RwLock;
use clap::{Arg, Command};
use serde_json::Value;
use colored::*;
use chrono::Local;
use rayon::prelude::*;
use local_ip_address::local_ip;
use crate::internal::chimera::{Config, AppState};
use crate::internal::port::find_available_port;
use crate::internal::json_data_generate::{JsonDataGeneratorSchema, generate_json_from_schema};

mod internal {
    pub mod chimera;
    pub mod port;
    pub mod json_data_generate;
}

async fn shutdown_signal() {
    // Create a future that resolves when Ctrl+C is pressed
    let ctrl_c = async {
        tokio::signal::ctrl_c()
            .await
            .expect("Failed to install Ctrl+C handler");
        println!("\nReceived shutdown signal, starting graceful shutdown...");
    };

    // On Windows, we need to handle ctrl_break differently
    #[cfg(windows)]
    let terminate = async {
        let mut ctrl_break = tokio::signal::windows::ctrl_break()
            .expect("Failed to install Ctrl+Break handler");
        ctrl_break.recv().await;
        println!("\nReceived Ctrl+Break signal, starting graceful shutdown...");
    };

    // On Unix, handle SIGTERM
    #[cfg(unix)]
    let terminate = async {
        let mut sigterm = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
            .expect("Failed to install SIGTERM handler");
        sigterm.recv().await;
        println!("\nReceived SIGTERM signal, starting graceful shutdown...");
    };

    // Wait for either signal
    tokio::select! {
        _ = ctrl_c => {},
        _ = terminate => {},
    }
}

async fn ping_pong() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain")],
        "status: ONLINE\nversion: 0.5.0\n🐲 All systems fused and breathing fire."
    )
}

async fn get_data(
    Path(route): Path<String>, 
    State(state): State<Arc<AppState>>, 
    uri: Uri
) -> Response {
    sleep(Duration::from_millis(state.latency)).await;

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Clone only the needed data immediately after acquiring lock
    let route_data = {
        let json_data = match timeout(Duration::from_millis(100), state.json_value.read()).await {
            Ok(lock) => lock.get(&route).cloned(),
            Err(_) => {
                log_request(&date_time, "500", "GET", &requested_path, true);
                return server_busy_response();
            },
        };
        json_data
    };

    match route_data {
        Some(mut value) => {
            // Rest of processing happens WITHOUT holding the lock
            if let Some((order, key)) = state.sort_rules.get(&route) {
                if let Value::Array(arr) = &mut value {
                    if arr.len() > 1 {
                        use rayon::slice::ParallelSliceMut;
                        arr.par_sort_by(|a, b| compare_values(a, b, key, order));
                    }
                }
            }

            if state.paginate > 0 {
                if let Value::Array(arr) = &value {
                    if arr.len() > state.paginate as usize {
                        value = Value::Array(arr[..state.paginate as usize].to_vec());
                    }
                }
            }

            log_request(&date_time, "200", "GET", &requested_path, false);
            (StatusCode::OK, axum::Json(value)).into_response()
        }
        None => {
            log_request(&date_time, "404", "GET", &requested_path, false);
            (StatusCode::NOT_FOUND, "Route not registered !!").into_response()
        },
    }
}

// Helper function for value comparison
fn compare_values(a: &Value, b: &Value, key: &str, order: &str) -> std::cmp::Ordering {
    let a_val = a.get(key).and_then(Value::as_i64).unwrap_or(0);
    let b_val = b.get(key).and_then(Value::as_i64).unwrap_or(0);
    if order == "asc" {
        a_val.cmp(&b_val)
    } else {
        b_val.cmp(&a_val)
    }
}

// Helper function for logging
fn log_request(date_time: &str, status: &str, method: &str, path: &str, is_error: bool) {
    let status_display = match status {
        "200" => " 200 ".bold().white().on_blue(),
        "404" => " 404 ".bold().white().on_red(),
        "500" => " 500 ".bold().white().on_green(),
        _ => " ??? ".bold().white().on_yellow(),
    };

    let method_display = match method {
        "GET" => " GET    ".bright_white().on_green(),
        _ => method.to_string().bright_white().on_green(),
    };

    println!(
        "|{}| {} |{}| {}",
        status_display,
        date_time.italic().dimmed(),
        method_display,
        path.italic()
    );
}

// Helper function for busy response
fn server_busy_response() -> Response {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        "Server is busy, try again later."
    ).into_response()
}

async fn get_data_by_id(
    Path((route, id)): Path<(String, String)>, 
    State(state): State<Arc<AppState>>, 
    uri: Uri
) -> Response {
    sleep(Duration::from_millis(state.latency)).await;

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    let json_data = match timeout(Duration::from_millis(100), state.json_value.read()).await {
        Ok(lock) => lock,
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server is busy, try again later."
            ).into_response();
        },
    };

    match json_data.get(&route) {
        Some(Value::Array(arr)) => {
            let _id = match id.parse::<i64>() {
                Ok(val) => val,
                Err(_) => return (StatusCode::BAD_REQUEST, "Invalid ID format").into_response(),
            };
            let record = arr.par_iter().find_any(|item| {
                item.get("id").and_then(Value::as_i64) == Some(_id)
            });

            match record {
                Some(record) => {
                    println!(
                        "|{}| {} |{}| {}",
                        " 200 ".bold().white().on_blue(),
                        date_time.italic().dimmed(),
                        " GET    ".bright_white().on_green(),
                        requested_path.italic()
                    );
                    return (StatusCode::OK, axum::Json(record)).into_response();
                },
                None => {
                    println!(
                        "|{}| {} |{}| {}",
                        " 404 ".bold().white().on_red(),
                        date_time.italic().dimmed(),
                        " GET    ".bright_white().on_green(),
                        requested_path.italic()
                    );
                    return (StatusCode::NOT_FOUND, "Record not found, check `id`").into_response();
                },
            }
        }
        Some(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 400 ".bold().white().on_yellow(),
                date_time.italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            return (StatusCode::BAD_REQUEST, "Route exists but is not an array.").into_response();
        },
        None => {
            println!(
                "|{}| {} |{}| {}",
                " 404 ".bold().white().on_red(),
                date_time.to_string().italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            return (StatusCode::NOT_FOUND, "Route not registered !!").into_response();
        },
    }
}

async fn delete_data(
    Path(route): Path<String>, 
    State(state): State<Arc<AppState>>, 
    uri: Uri
) -> Response {

    sleep(Duration::from_millis(state.latency)).await;

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    let mut json_data = match timeout(Duration::from_millis(100), state.json_value.write()).await {
        Ok(lock) => lock, 
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server is busy, try again later."
            ).into_response();
        },
    };

    match json_data.get_mut(&route) {
        Some(Value::Array(arr)) => {
            arr.clear();
            println!(
                "|{}| {} |{}| {}",
                " 200 ".bold().white().on_blue(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (StatusCode::OK, "All records deleted successfully").into_response();
        }
        Some(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 400 ".bold().white().on_yellow(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (StatusCode::BAD_REQUEST, "Route exists but is not an array.").into_response();
        },
        None => {
            println!(
                "|{}| {} |{}| {}",
                " 404 ".bold().white().on_red(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (StatusCode::NOT_FOUND, "Route not registered !!").into_response();
        },
    }
}

async fn delete_data_by_id(
    Path((route, id)): Path<(String, String)>, 
    State(state): State<Arc<AppState>>, 
    uri: Uri
) -> Response {
    sleep(Duration::from_millis(state.latency)).await;

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();
    
    // Use write lock for exclusive access during modification
    let mut json_data = match timeout(Duration::from_millis(100), state.json_value.write()).await {
        Ok(lock) => lock, 
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Server is busy, try again later."
            ).into_response();
        },
    };

    let _id = match id.parse::<i64>() {
        Ok(value) => value,
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 400 ".bold().white().on_yellow(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (StatusCode::BAD_REQUEST, "Invalid ID format").into_response();
        },
    };

    match json_data.get_mut(&route) {
        Some(Value::Array(arr)) => {
            let initial_len = arr.len();
            
            // Use parallel processing only for large arrays
            if arr.len() > 100 {
                use rayon::prelude::*;
                let mut filtered: Vec<Value> = arr.par_iter()
                    .filter(|item| item.get("id").and_then(Value::as_i64) != Some(_id))
                    .cloned()
                    .collect();
                *arr = filtered;
            } else {
                arr.retain(|item| item.get("id").and_then(Value::as_i64) != Some(_id));
            }

            if arr.len() < initial_len {
                println!(
                    "|{}| {} |{}| {}",
                    " 200 ".bold().white().on_blue(),
                    date_time.italic().dimmed(),
                    " DELETE ".bright_white().on_red(),
                    requested_path.italic()
                );
                return (StatusCode::OK, "Record deleted successfully").into_response();
            } else {
                println!(
                    "|{}| {} |{}| {}",
                    " 404 ".bold().white().on_red(),
                    date_time.italic().dimmed(),
                    " DELETE ".bright_white().on_red(),
                    requested_path.italic()
                );
                return (StatusCode::NOT_FOUND, "ID not found in array").into_response();
            }
        }
        Some(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 400 ".bold().white().on_yellow(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (StatusCode::BAD_REQUEST, "Route exists but is not an array.").into_response();
        },
        None => {
            println!(
                "|{}| {} |{}| {}",
                " 404 ".bold().white().on_red(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return (StatusCode::NOT_FOUND, "Route not registered !!").into_response();
        },
    }
}

async fn run_axum_server(config: Config) -> Result<(), IOError> {
    // Create AppState from Config
    let state = Arc::new(AppState {
        path: config.path,
        port: config.port,
        json_value: config.json_value,
        latency: config.latency,
        sort_rules: config.sort_rules,
        paginate: config.paginate,
    });

    // Build router with Axum
    let app = Router::new()
        .route("/", get(ping_pong))
        .route("/:route", get(get_data))
        .route("/:route", delete(delete_data))
        .route("/:route/:id", get(get_data_by_id))
        .route("/:route/:id", delete(delete_data_by_id))
        .with_state(state.clone());

    // Address to bind the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    // Display server info
    let local = "127.0.0.1";
    let lan_ip = local_ip().unwrap_or_else(|_| local.parse().unwrap());
    println!(" - Local:               http://{}:{}", local, config.port);
    println!(" - Network:             http://{}:{}\n", lan_ip, config.port);
    
    // Setup graceful shutdown
    let server = Server::bind(&addr)
        .serve(app.into_make_service());
    
    // Create a future that completes when a shutdown signal is received
    let graceful = server.with_graceful_shutdown(shutdown_signal());
    
    // Wait for the server to complete (or for a shutdown signal)
    if let Err(e) = graceful.await {
        eprintln!("Server error: {}", e);
    } else {
        println!("Server shutdown complete");
    }

    Ok(())
}

async fn run_grpc_server(config: Config) -> Result<(), IOError> {
    println!("{:#?}", config);

    Ok(())
}

fn initialize_cmd() -> Result<Config, IOError> {
    let matches = Command::new("Chimera - JSoN SeRVeR")
        .version("0.5.0")
        .author("Abhijith M S")
        .about("A powerful and fast⚡ JSoN SeRVeR built in Rust 🦀")
        .arg(Arg::new("port")
            .short('p')
            .long("port")
            .num_args(1)
            .default_value("8080")
            .help("Port for the server"))
        .arg(Arg::new("path")
            .short('P')
            .long("path")
            .num_args(1)
            .required(true)
            .help("Path to the Json file"))
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
        .arg(Arg::new("protocol")
            .short('Z')
            .long("protocol")
            .num_args(1)
            .default_value("http")
            .help("The protocol to use for the Mock API"))
        .get_matches();

    let json_file_path = matches.get_one::<String>("path").expect("Missing path argument").to_string();
    let api_protocol = matches.get_one::<String>("protocol").expect("Invalid protocol").to_string();
    let server_port = matches.get_one::<String>("port").unwrap().parse::<u16>().expect("Invalid port number");
    let sim_latency_str = matches.get_one::<String>("latency").expect("Missing latency argument").to_string();
    let sim_latency: u64 = sim_latency_str.trim_end_matches("ms").parse::<u64>().expect("Invalid latency format");
    let pagination_factor = matches.get_one::<String>("page").unwrap().parse::<u64>().expect("Invalid page format");
    let auto_generate_enabled = matches.get_flag("auto_generate_data");

    let json_content = fs::read_to_string(&json_file_path).expect("Failed to read Json File");

    let parsed_content: Value = if auto_generate_enabled {
        let schema: JsonDataGeneratorSchema = serde_json::from_str(&json_content).expect("Invalid schema format for auto data generation");
        let result = generate_json_from_schema(schema);
        result
    } else {
        let content: Value = serde_json::from_str(&json_content).expect("Invalid Json format");
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
            eprintln!("Error: The given json file is a JSON Array! It should be a JSON Object");
            process::exit(1);
        }
        content
    };

    let mut sort_rules: HashMap<String, (String, String)> = HashMap::new();
    if let Some(sort_args) = matches.get_many::<String>("sort") {
        let sort_list: Vec<String> = sort_args.map(|s| s.clone()).collect();
        for sort_group in sort_list.chunks(3) {
            if let [route, order, key] = sort_group {
                sort_rules.insert(route.clone(), (order.clone(), key.clone()));
            }
        }
    }
    
    let final_port: u16 = find_available_port(server_port);

    Ok(Config {
        path: json_file_path,
        port: final_port,
        mode: api_protocol,
        json_value: Arc::new(RwLock::new(parsed_content)),
        latency: sim_latency,
        sort_rules: sort_rules,
        paginate: pagination_factor,
    })
}

#[tokio::main]
async fn main() -> Result<(), IOError> {
    println!("
╔═╗┬ ┬┬┌┬┐┌─┐┬─┐┌─┐
║  ├─┤││││├┤ ├┬┘├─┤
╚═╝┴ ┴┴┴ ┴└─┘┴└─┴ ┴
v0.5.0
    ");

    let config_data = initialize_cmd()?;

    match config_data.mode.as_str() {
        "http" => {
            if let Err(e) = run_axum_server(config_data).await {
                eprintln!("Failed to run Axum server: {}", e);
            }
        },
        "grpc" => {
            if let Err(e) = run_grpc_server(config_data).await {
                eprintln!("Failed to run Tonic server: {}", e);
            }
        },
        _ => {
            println!("PROTOCOL NOT SUPPORTED !!");
        }
    }

    println!("
Chimera retreats to the shadows... 
It will rise again. 🐉
    ");
    Ok(())
}