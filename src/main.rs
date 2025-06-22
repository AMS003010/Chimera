use crate::internal::chimera::{AppState, Config, CHIMERA_LATEST_VERSION};
use crate::internal::helpers::{
    compare_values, find_key_and_id_lengths, log_request, server_busy_response, shutdown_signal,
};
use crate::internal::json_data_generate::{generate_json_from_schema, JsonDataGeneratorSchema};
use crate::internal::port::find_available_port;
use axum::{
    extract::{Path, State},
    http::{StatusCode, Uri},
    response::{IntoResponse, Response},
    routing::{delete, get, put, patch, post},
    Router,
    Json,
};
use chrono::Local;
use clap::{Arg, Command};
use hyper::server::Server;
use local_ip_address::local_ip;
use serde_json::Value;
use std::collections::HashMap;
use std::io::Error as IOError;
use std::process;
use std::sync::Arc;
use std::time::Instant;
use std::{fs, net::SocketAddr};
use tokio::sync::RwLock;
use tokio::time::{sleep, timeout, Duration};

mod internal {
    pub mod chimera;
    pub mod helpers;
    pub mod json_data_generate;
    pub mod port;
}

async fn ping_pong() -> impl IntoResponse {
    (
        StatusCode::OK,
        [("content-type", "text/plain")],
        format!(
            "status: ONLINE\nversion: {}\n🐲 All systems fused and breathing fire.",
            CHIMERA_LATEST_VERSION
        )
        .to_string(),
    )
}

async fn get_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    sleep(Duration::from_millis(state.latency)).await;

    // Clone only the needed data immediately after acquiring lock
    let route_data = {
        let json_data = match timeout(Duration::from_millis(100), state.json_value.read()).await {
            Ok(lock) => {
                if let Some(path_id) = route.split("/").last() {
                    if let Ok(_id) = path_id.parse::<usize>() {
                        let mut route_parts: Vec<&str> = route.split('/').collect();
                        route_parts.pop();
                        let base_path = route_parts.join("/");
                        lock.get(&base_path).cloned()
                    } else {
                        lock.get(&route).cloned()
                    }
                } else {
                    lock.get(&route).cloned()
                }
            }
            Err(_) => {
                let elapsed = start_time.elapsed().as_millis();
                log_request(
                    &date_time,
                    "500",
                    "GET",
                    &requested_path,
                    true,
                    elapsed,
                    state.max_request_path_len,
                    state.max_request_path_id_length,
                    0,
                );
                return server_busy_response();
            }
        };
        json_data
    };

    match route_data {
        Some(mut value) => {
            if let Some(path_id) = route.split("/").last() {
                if let Ok(_id) = path_id.parse::<usize>() {
                    if let Value::Array(ref mut arr) = value {
                        arr.retain(|obj| {
                            obj.get("id")
                                .and_then(|id| id.as_u64())
                                .map_or(false, |id_num| id_num == _id as u64)
                        });
                    }
                    let elapsed = start_time.elapsed().as_millis();
                    log_request(
                        &date_time,
                        "200",
                        "GET",
                        &requested_path,
                        false,
                        elapsed,
                        state.max_request_path_len,
                        state.max_request_path_id_length,
                        value.as_array().map_or(0, |arr| arr.len()),
                    );
                    return (StatusCode::OK, axum::Json(value)).into_response();
                }
            }

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

            let elapsed = start_time.elapsed().as_millis();
            log_request(
                &date_time,
                "200",
                "GET",
                &requested_path,
                false,
                elapsed,
                state.max_request_path_len,
                state.max_request_path_id_length,
                value.as_array().map_or(0, |arr| arr.len()),
            );
            (StatusCode::OK, axum::Json(value)).into_response()
        }
        None => {
            let elapsed = start_time.elapsed().as_millis();
            log_request(
                &date_time,
                "404",
                "GET",
                &requested_path,
                false,
                elapsed,
                state.max_request_path_len,
                state.max_request_path_id_length,
                0,
            );
            (StatusCode::NOT_FOUND, "Route not registered !!").into_response()
        }
    }
}

async fn delete_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    sleep(Duration::from_millis(state.latency)).await;

    // Handle the DELETE operation
    let delete_result = {
        let mut json_data =
            match timeout(Duration::from_millis(100), state.json_value.write()).await {
                Ok(lock) => lock,
                Err(_) => {
                    let elapsed = start_time.elapsed().as_millis();
                    log_request(
                        &date_time,
                        "500",
                        "DELETE",
                        &requested_path,
                        true,
                        elapsed,
                        state.max_request_path_len,
                        state.max_request_path_id_length,
                        0,
                    );
                    return server_busy_response();
                }
            };

        // Check if we're deleting a specific ID
        if let Some(path_id) = route.split("/").last() {
            if let Ok(id) = path_id.parse::<usize>() {
                // Extract base path (remove the ID part)
                let mut route_parts: Vec<&str> = route.split('/').collect();
                route_parts.pop();
                let base_path = route_parts.join("/");

                match json_data.get_mut(&base_path) {
                    Some(Value::Array(arr)) => {
                        let original_len = arr.len();
                        arr.retain(|obj| {
                            obj.get("id")
                                .and_then(|id_val| id_val.as_u64())
                                .map_or(true, |id_num| id_num != id as u64)
                        });
                        let deleted_count = original_len - arr.len();

                        if deleted_count > 0 {
                            (
                                "200",
                                format!("Deleted {} record(s) with id {}", deleted_count, id),
                                deleted_count,
                            )
                        } else {
                            ("404", format!("No record found with id {}", id), 0)
                        }
                    }
                    Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                    None => ("404", "Route not registered !!".to_string(), 0),
                }
            } else {
                // Delete entire collection
                match json_data.get_mut(&route) {
                    Some(Value::Array(arr)) => {
                        let deleted_count = arr.len();
                        arr.clear();
                        (
                            "200",
                            "All records deleted successfully".to_string(),
                            deleted_count,
                        )
                    }
                    Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                    None => ("404", "Route not registered !!".to_string(), 0),
                }
            }
        } else {
            // Delete entire collection
            match json_data.get_mut(&route) {
                Some(Value::Array(arr)) => {
                    let deleted_count = arr.len();
                    arr.clear();
                    (
                        "200",
                        "All records deleted successfully".to_string(),
                        deleted_count,
                    )
                }
                Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                None => ("404", "Route not registered !!".to_string(), 0),
            }
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = delete_result;

    // Log the request
    log_request(
        &date_time,
        status_code,
        "DELETE",
        &requested_path,
        false,
        elapsed,
        state.max_request_path_len,
        state.max_request_path_id_length,
        affected_records,
    );

    // Return appropriate response
    match status_code {
        "200" => (StatusCode::OK, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "404" => (StatusCode::NOT_FOUND, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
    }
}


// POST handler - Create new records
async fn post_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    sleep(Duration::from_millis(state.latency)).await;

    // Handle the POST operation
    let post_result = {
        let mut json_data = match timeout(Duration::from_millis(100), state.json_value.write()).await {
            Ok(lock) => lock,
            Err(_) => {
                let elapsed = start_time.elapsed().as_millis();
                log_request(
                    &date_time,
                    "500",
                    "POST",
                    &requested_path,
                    true,
                    elapsed,
                    state.max_request_path_len,
                    state.max_request_path_id_length,
                    0,
                );
                return server_busy_response();
            }
        };

        if let Value::Object(ref mut obj) = *json_data {
            match obj.get_mut(&route) {
                Some(Value::Array(arr)) => {
                    match payload {
                        Value::Array(new_items) => {
                            let added_count = new_items.len();
                            arr.extend(new_items);
                            ("201", format!("Added {} record(s) successfully", added_count), added_count)
                        }
                        single_item => {
                            arr.push(single_item);
                            ("201", "Record added successfully".to_string(), 1)
                        }
                    }
                }
                Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                None => {
                    // Create new route with the payload
                    match payload {
                        Value::Array(items) => {
                            let added_count = items.len();
                            obj.insert(route.clone(), Value::Array(items));
                            ("201", format!("Created route and added {} record(s)", added_count), added_count)
                        }
                        single_item => {
                            obj.insert(route.clone(), Value::Array(vec![single_item]));
                            ("201", "Created route and added record".to_string(), 1)
                        }
                    }
                }
            }
        } else {
            ("500", "Root JSON is not an object".to_string(), 0)
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = post_result;
    
    log_request(
        &date_time,
        status_code,
        "POST",
        &requested_path,
        false,
        elapsed,
        state.max_request_path_len,
        state.max_request_path_id_length,
        affected_records,
    );

    match status_code {
        "201" => (StatusCode::CREATED, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "500" => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
    }
}

// PUT handler - Replace entire resource or create if not exists
async fn put_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    sleep(Duration::from_millis(state.latency)).await;

    // Handle the PUT operation
    let put_result = {
        let mut json_data = match timeout(Duration::from_millis(100), state.json_value.write()).await {
            Ok(lock) => lock,
            Err(_) => {
                let elapsed = start_time.elapsed().as_millis();
                log_request(
                    &date_time,
                    "500",
                    "PUT",
                    &requested_path,
                    true,
                    elapsed,
                    state.max_request_path_len,
                    state.max_request_path_id_length,
                    0,
                );
                return server_busy_response();
            }
        };

        if let Value::Object(ref mut obj) = *json_data {
            // Check if we're updating a specific ID
            if let Some(path_id) = route.split("/").last() {
                if let Ok(id) = path_id.parse::<usize>() {
                    // Extract base path (remove the ID part)
                    let mut route_parts: Vec<&str> = route.split('/').collect();
                    route_parts.pop();
                    let base_path = route_parts.join("/");
                    
                    match obj.get_mut(&base_path) {
                        Some(Value::Array(arr)) => {
                            let mut found = false;
                            for item in arr.iter_mut() {
                                if let Some(item_id) = item.get("id").and_then(|id| id.as_u64()) {
                                    if item_id == id as u64 {
                                        *item = payload.clone();
                                        // Ensure the ID is preserved
                                        if let Value::Object(ref mut item_obj) = item {
                                            item_obj.insert("id".to_string(), Value::Number(serde_json::Number::from(id)));
                                        }
                                        found = true;
                                        break;
                                    }
                                }
                            }
                            
                            if found {
                                ("200", format!("Updated record with id {}", id), 1)
                            } else {
                                // Create new record with the specified ID
                                let mut new_item = payload.clone();
                                if let Value::Object(ref mut item_obj) = new_item {
                                    item_obj.insert("id".to_string(), Value::Number(serde_json::Number::from(id)));
                                }
                                arr.push(new_item);
                                ("201", format!("Created record with id {}", id), 1)
                            }
                        }
                        Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                        None => ("404", "Route not registered !!".to_string(), 0),
                    }
                } else {
                    // Replace entire collection
                    let record_count = match &payload {
                        Value::Array(arr) => arr.len(),
                        _ => 1,
                    };
                    let was_existing = obj.contains_key(&route);
                    obj.insert(route.clone(), payload);
                    
                    if was_existing {
                        ("200", format!("Replaced entire collection with {} record(s)", record_count), record_count)
                    } else {
                        ("201", format!("Created collection with {} record(s)", record_count), record_count)
                    }
                }
            } else {
                // Replace entire collection
                let record_count = match &payload {
                    Value::Array(arr) => arr.len(),
                    _ => 1,
                };
                let was_existing = obj.contains_key(&route);
                obj.insert(route.clone(), payload);
                
                if was_existing {
                    ("200", format!("Replaced entire collection with {} record(s)", record_count), record_count)
                } else {
                    ("201", format!("Created collection with {} record(s)", record_count), record_count)
                }
            }
        } else {
            ("500", "Root JSON is not an object".to_string(), 0)
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = put_result;
    
    log_request(
        &date_time,
        status_code,
        "PUT",
        &requested_path,
        false,
        elapsed,
        state.max_request_path_len,
        state.max_request_path_id_length,
        affected_records,
    );

    match status_code {
        "200" => (StatusCode::OK, message).into_response(),
        "201" => (StatusCode::CREATED, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "404" => (StatusCode::NOT_FOUND, message).into_response(),
        "500" => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
    }
}

// PATCH handler - Partial update of existing resource
async fn patch_data(
    Path(route): Path<String>,
    State(state): State<Arc<AppState>>,
    uri: Uri,
    Json(payload): Json<Value>,
) -> Response {
    let start_time = Instant::now();

    // Get these before locking
    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = uri.path();

    // Add the Latency
    sleep(Duration::from_millis(state.latency)).await;

    // Handle the PATCH operation
    let patch_result = {
        let mut json_data = match timeout(Duration::from_millis(100), state.json_value.write()).await {
            Ok(lock) => lock,
            Err(_) => {
                let elapsed = start_time.elapsed().as_millis();
                log_request(
                    &date_time,
                    "500",
                    "PATCH",
                    &requested_path,
                    true,
                    elapsed,
                    state.max_request_path_len,
                    state.max_request_path_id_length,
                    0,
                );
                return server_busy_response();
            }
        };

        if let Value::Object(ref mut obj) = *json_data {
            // Check if we're updating a specific ID
            if let Some(path_id) = route.split("/").last() {
                if let Ok(id) = path_id.parse::<usize>() {
                    // Extract base path (remove the ID part)
                    let mut route_parts: Vec<&str> = route.split('/').collect();
                    route_parts.pop();
                    let base_path = route_parts.join("/");
                    
                    match obj.get_mut(&base_path) {
                        Some(Value::Array(arr)) => {
                            let mut found = false;
                            for item in arr.iter_mut() {
                                if let Some(item_id) = item.get("id").and_then(|id| id.as_u64()) {
                                    if item_id == id as u64 {
                                        // Merge the payload into the existing item
                                        if let (Value::Object(existing), Value::Object(updates)) = (item, &payload) {
                                            for (key, value) in updates {
                                                existing.insert(key.clone(), value.clone());
                                            }
                                            found = true;
                                            break;
                                        }
                                    }
                                }
                            }
                            
                            if found {
                                ("200", format!("Partially updated record with id {}", id), 1)
                            } else {
                                ("404", format!("No record found with id {}", id), 0)
                            }
                        }
                        Some(_) => ("400", "Route exists but is not an array.".to_string(), 0),
                        None => ("404", "Route not registered !!".to_string(), 0),
                    }
                } else {
                    ("400", "PATCH requires a specific resource ID".to_string(), 0)
                }
            } else {
                ("400", "PATCH requires a specific resource ID".to_string(), 0)
            }
        } else {
            ("500", "Root JSON is not an object".to_string(), 0)
        }
    };

    let elapsed = start_time.elapsed().as_millis();
    let (status_code, message, affected_records) = patch_result;
    
    log_request(
        &date_time,
        status_code,
        "PATCH",
        &requested_path,
        false,
        elapsed,
        state.max_request_path_len,
        state.max_request_path_id_length,
        affected_records,
    );

    match status_code {
        "200" => (StatusCode::OK, message).into_response(),
        "400" => (StatusCode::BAD_REQUEST, message).into_response(),
        "404" => (StatusCode::NOT_FOUND, message).into_response(),
        "500" => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
        _ => (StatusCode::INTERNAL_SERVER_ERROR, message).into_response(),
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
        max_request_path_id_length: config.max_request_path_id_length,
        max_request_path_len: config.max_request_path_len,
    });

    // Build router with Axum
    let app = Router::new()
        .route("/", get(ping_pong))
        .route("/*route", get(get_data))
        .route("/*route", delete(delete_data))
        .route("/*route", post(post_data))
        .route("/*route", put(put_data))
        .route("/*route", patch(patch_data))
        .with_state(state.clone());

    // Address to bind the server
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));

    // Display server info
    let local = "127.0.0.1";
    let lan_ip = local_ip().unwrap_or_else(|_| local.parse().unwrap());
    println!(" - Local:     http://{}:{}", local, config.port);
    println!(" - Network:   http://{}:{}\n", lan_ip, config.port);

    // Setup graceful shutdown
    let server = Server::bind(&addr).serve(app.into_make_service());

    // Create a future that completes when a shutdown signal is received
    let graceful = server.with_graceful_shutdown(shutdown_signal());

    // Wait for the server to complete (or for a shutdown signal)
    if let Err(e) = graceful.await {
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

fn initialize_cmd() -> Result<Config, IOError> {
    let matches = Command::new("Chimera - JSoN SeRVeR")
        .version(CHIMERA_LATEST_VERSION)
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

    let json_file_path = matches
        .get_one::<String>("path")
        .expect("Missing path argument")
        .to_string();
    let api_protocol = matches
        .get_one::<String>("protocol")
        .expect("Invalid protocol")
        .to_string();
    let server_port = matches
        .get_one::<String>("port")
        .unwrap()
        .parse::<u16>()
        .expect("Invalid port number");
    let sim_latency_str = matches
        .get_one::<String>("latency")
        .expect("Missing latency argument")
        .to_string();
    let sim_latency: u64 = sim_latency_str
        .trim_end_matches("ms")
        .parse::<u64>()
        .expect("Invalid latency format");
    let pagination_factor = matches
        .get_one::<String>("page")
        .unwrap()
        .parse::<u64>()
        .expect("Invalid page format");
    let auto_generate_enabled = matches.get_flag("auto_generate_data");

    let json_content = fs::read_to_string(&json_file_path).expect("Failed to read Json File");

    let parsed_content: Value = if auto_generate_enabled {
        let schema: JsonDataGeneratorSchema = serde_json::from_str(&json_content)
            .expect("Invalid schema format for auto data generation");
        let result = generate_json_from_schema(schema);
        result
    } else {
        let content: Value = serde_json::from_str(&json_content).expect("Invalid Json format");
        // println!("{:#?}", content);
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

    let mut spaces = 0;
    let mut longest_path = 0;

    if let Some((key, len)) = find_key_and_id_lengths(&parsed_content) {
        spaces = len;
        longest_path = key;
    }

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
        max_request_path_id_length: spaces,
        max_request_path_len: longest_path,
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

    let config_data = initialize_cmd()?;

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
