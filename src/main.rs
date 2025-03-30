use std::io::Error as IOError;
use std::process;
use std::collections::HashMap;
use std::{fs, sync::Mutex};
use actix_web::{web, App, get, post, delete, Responder, HttpServer, HttpResponse};
use tokio::time::{sleep, Duration};
use clap::{Arg, Command};
use serde_json::Value;
use colored::*;
use chrono::Local;

use internal::port::find_available_port;
use internal::chimera::Config;

mod internal {
    pub mod chimera;
    pub mod port;
}

async fn ping_pong() -> impl Responder {
    HttpResponse::Ok().content_type("text/html").body("Pong 🏓")
}

#[get("/{route}")]
async fn get_data(path: web::Path<String>, data: web::Data<Config>, req: actix_web::HttpRequest) -> impl Responder {
    sleep(Duration::from_millis(data.latency)).await;

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = req.path();

    let json_data = match data.json_value.try_lock() {
        Ok(lock) => lock,
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            return HttpResponse::InternalServerError().body("Server is busy, try again later.")
        },
    };

    let route = path.into_inner();

    match json_data.get(&route) {
        Some(value) => {
            let mut sorted_data = value.clone();

            if let Some((order, key)) = data.sort_rules.get(&route) {
                if let Value::Array(arr) = &mut sorted_data {
                    arr.sort_by(|a, b| {
                        let a_val = a.get(key).and_then(Value::as_i64).unwrap_or(0);
                        let b_val = b.get(key).and_then(Value::as_i64).unwrap_or(0);
                        if order == "asc" {
                            a_val.cmp(&b_val)
                        } else {
                            b_val.cmp(&a_val)
                        }
                    });
                }
            }

            if data.paginate > 0 {
                if let Value::Array(arr) = &sorted_data {
                    if arr.len() > data.paginate as usize {
                        sorted_data = Value::Array(
                            arr.iter()
                                .take(data.paginate.try_into().unwrap_or(usize::MAX))
                                .cloned()
                                .collect(),
                        );
                    }
                }
                println!(
                    "|{}| {} |{}| {}",
                    " 200 ".bold().white().on_blue(),
                    date_time.italic().dimmed(),
                    " GET    ".bright_white().on_green(),
                    requested_path.italic()
                );
                return HttpResponse::Ok().json(sorted_data);
            }
            println!(
                "|{}| {} |{}| {}",
                " 200 ".bold().white().on_blue(),
                date_time.italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            return HttpResponse::Ok().json(sorted_data);
        }
        None => {
            println!(
                "|{}| {} |{}| {}",
                " 404 ".bold().white().on_red(),
                date_time.italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            HttpResponse::NotFound().body("Route not found")
        },
    }
}

#[get("/{route}/{id}")]
async fn get_data_by_id(path: web::Path<(String, String)>, data: web::Data<Config>, req: actix_web::HttpRequest) -> impl Responder {
    sleep(Duration::from_millis(data.latency)).await;

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = req.path();

    let json_data = match data.json_value.try_lock() {
        Ok(lock) => lock,
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            return HttpResponse::InternalServerError().body("Server is busy, try again later.")
        },
    };

    let (route, id) = path.into_inner();

    match json_data.get(&route) {
        Some(Value::Array(arr)) => {
            let _id = id.parse::<i64>().expect("Invalid ID");
            let record = arr.iter().find(|item| {
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
                    HttpResponse::Ok().json(record)
                },
                None => {
                    println!(
                        "|{}| {} |{}| {}",
                        " 404 ".bold().white().on_red(),
                        date_time.italic().dimmed(),
                        " GET    ".bright_white().on_green(),
                        requested_path.italic()
                    );
                    HttpResponse::NotFound().body("Record not found, check `id`.")
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
            HttpResponse::BadRequest().body("Route exists but is not an array.")
        },
        None => {
            println!(
                "|{}| {} |{}| {}",
                " 404 ".bold().white().on_red(),
                date_time.to_string().italic().dimmed(),
                " GET    ".bright_white().on_green(),
                requested_path.italic()
            );
            HttpResponse::NotFound().body("Route not found.")
        },
    }
}

#[delete("/{route}")]
async fn delete_data(path: web::Path<String>,data: web::Data<Config>, req: actix_web::HttpRequest) -> impl Responder {

    sleep(Duration::from_millis(data.latency)).await;

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = req.path();

    let mut json_data = match data.json_value.try_lock() {
        Ok(lock) => lock, 
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return HttpResponse::InternalServerError().body("Server is busy, try again later.")
        },
    };

    let route = path.into_inner();

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
            HttpResponse::Ok().body("All records deleted successfully")
        }
        Some(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 400 ".bold().white().on_yellow(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            HttpResponse::BadRequest().body("Route exists but is not an array.")
        },
        None => {
            println!(
                "|{}| {} |{}| {}",
                " 404 ".bold().white().on_red(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            HttpResponse::NotFound().body("Route not found.")
        },
    }
}

#[delete("/{route}/{id}")]
async fn delete_data_by_id(path: web::Path<(String, String)>,data: web::Data<Config>, req: actix_web::HttpRequest) -> impl Responder {

    sleep(Duration::from_millis(data.latency)).await;

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = req.path();
    
    let mut json_data = match data.json_value.try_lock() {
        Ok(lock) => lock, 
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            return HttpResponse::InternalServerError().body("Server is busy, try again later.")
        },
    };

    let (route, id) = path.into_inner();
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
            return HttpResponse::BadRequest().body("Invalid ID format")
        },
    };

    match json_data.get_mut(&route) {
        Some(Value::Array(arr)) => {
            let initial_len = arr.len();
            
            arr.retain(|item| item.get("id").and_then(Value::as_i64) != Some(_id));

            if arr.len() < initial_len {
                println!(
                    "|{}| {} |{}| {}",
                    " 200 ".bold().white().on_blue(),
                    date_time.italic().dimmed(),
                    " DELETE ".bright_white().on_red(),
                    requested_path.italic()
                );
                HttpResponse::Ok().body("Record deleted successfully")
            } else {
                println!(
                    "|{}| {} |{}| {}",
                    " 404 ".bold().white().on_red(),
                    date_time.italic().dimmed(),
                    " DELETE ".bright_white().on_red(),
                    requested_path.italic()
                );
                HttpResponse::NotFound().body("Record not found, check `id`.")
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
            HttpResponse::BadRequest().body("Route exists but is not an array.")
        },
        None => {
            println!(
                "|{}| {} |{}| {}",
                " 404 ".bold().white().on_red(),
                date_time.italic().dimmed(),
                " DELETE ".bright_white().on_red(),
                requested_path.italic()
            );
            HttpResponse::NotFound().body("Route not found.")
        },
    }
}

#[post("/{route}")]
async fn add_data(path: web::Path<String>, body: web::Json<Value>, data: web::Data<Config>, req: actix_web::HttpRequest) -> impl Responder {

    let now = Local::now();
    let date_time = now.format("%Y/%m/%d - %H:%M:%S").to_string();
    let requested_path = req.path();
    let route = path.into_inner();
    let new_entry = body.into_inner();

    let mut json_data = match data.json_value.try_lock() {
        Ok(lock) => lock,
        Err(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 500 ".bold().white().on_green(),
                date_time.italic().dimmed(),
                " POST   ".bright_red().on_white(),
                requested_path.italic()
            );
            return HttpResponse::InternalServerError().body("Server is busy, try again later.")
        },
    };

    match new_entry {
        Value::Object(obj) => {
            match json_data.as_object_mut() {
                Some(map) => {
                    if let Some(Value::Array(arr)) = map.get_mut(&route) {
                        arr.push(Value::Object(obj.clone()));
                    } else {
                        map.insert(route.clone(), Value::Array(vec![Value::Object(obj.clone())]));
                    }

                    println!(
                        "|{}| {} |{}| {}",
                        " 201 ".bold().white().on_cyan(),
                        date_time.italic().dimmed(),
                        " POST   ".bright_red().on_white(),
                        requested_path.italic()
                    );
                    HttpResponse::Created().json(obj)
                }
                None => {
                    println!(
                        "|{}| {} |{}| {}",
                        " 500 ".bold().white().on_red(),
                        date_time.italic().dimmed(),
                        " POST   ".bright_red().on_white(),
                        requested_path.italic()
                    );
                    HttpResponse::InternalServerError().body("Internal JSON structure error.")
                }
            }
        }
        Value::Array(_) => {
            println!(
                "|{}| {} |{}| {}",
                " 400 ".bold().white().on_yellow(),
                date_time.italic().dimmed(),
                " POST   ".bright_red().on_white(),
                requested_path.italic()
            );
            HttpResponse::BadRequest().body("Cannot post an array. Send an object instead.")
        }
        _ => {
            println!(
                "|{}| {} |{}| {}",
                " 400 ".bold().white().on_yellow(),
                date_time.italic().dimmed(),
                " POST   ".bright_red().on_white(),
                requested_path.italic()
            );
            HttpResponse::BadRequest().body("Body has illegal JSON format.")
        }
    }
}

async fn run_actix_server() -> Result<(), IOError> {
    
    let matches = Command::new("Chimera - JSoN SeRVeR")
        .version("0.1.0")
        .author("Abhijith M S")
        .about("A powerful and fast⚡ Json server built in Rust 🦀")
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
        .get_matches();

    let json_file_path = matches.get_one::<String>("path").expect("Missing path argument").to_string();
    let server_port = matches.get_one::<String>("port").unwrap().parse::<u16>().expect("Invalid port number");
    let sim_latency_str = matches.get_one::<String>("latency").expect("Missing latency argument").to_string();
    let sim_latency: u64 = sim_latency_str.trim_end_matches("ms").parse::<u64>().expect("Invalid latency format");
    let pagination_factor = matches.get_one::<String>("page").unwrap().parse::<u64>().expect("Invalid page format");

    let json_content = fs::read_to_string(&json_file_path).expect("Failed to read Json File");
    let parsed_content: Value = serde_json::from_str(&json_content).expect("Invalid Json format");

    let json_type = match &parsed_content {
        Value::Object(_) => true,
        Value::Array(_) => false,
        _ => false
    };

    if json_type == false {
        eprintln!("Error: The given json file is a JSON Array !! It should be a JSON Object");
        process::exit(1);
    }

    let final_port: u16 = find_available_port(server_port);

    let mut sort_rules: HashMap<String, (String, String)> = HashMap::new();
    if let Some(sort_args) = matches.get_many::<String>("sort") {
        let sort_list: Vec<String> = sort_args.map(|s| s.clone()).collect();
        for sort_group in sort_list.chunks(3) {
            if let [route, order, key] = sort_group {
                sort_rules.insert(route.clone(), (order.clone(), key.clone()));
            }
        }
    }
    
    let config_data = web::Data::new(Config {
        path: json_file_path,
        port: final_port,
        json_value: Mutex::new(parsed_content),
        latency: sim_latency,
        sort_rules: sort_rules,
        paginate: pagination_factor,
    });

    println!("🔱 Chimera JSON Server running at http://127.0.0.1:{}\n", final_port);

    let server = HttpServer::new(move || {
        App::new()
            .app_data(config_data.clone())
            .route("/", web::get().to(ping_pong))
            .service(get_data)
            .service(get_data_by_id)
            .service(add_data)
            .service(delete_data)
            .service(delete_data_by_id)
    })
    .bind(format!("0.0.0.0:{}", final_port))?
    .workers(4)
    .shutdown_timeout(60)
    .run();

    server.await?;
    Ok(())
}

#[actix_web::main]
async fn main() -> Result<(), IOError> {
    println!("
╔═╗┬ ┬┬┌┬┐┌─┐┬─┐┌─┐
║  ├─┤││││├┤ ├┬┘├─┤
╚═╝┴ ┴┴┴ ┴└─┘┴└─┴ ┴
v0.1.0
    ");

    if let Err(e) = run_actix_server().await {
        eprintln!("Failed to run Actix server: {}", e);
    }

    println!("
Chimera retreats to the shadows... 
It will rise again. 🐉
    ");
    Ok(())
}
