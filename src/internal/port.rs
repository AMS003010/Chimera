use std::net::{TcpListener, TcpStream};
use colored::Colorize;

pub fn find_available_port(mut port: u16) -> u16 {
    println!("[{}] Trying to bind to port {}", "INFO".green(), port);
    loop {
        if is_port_ok(port) {
            println!("[{}] Port {} is available", "INFO".green(), port);
            return port;
        }
        port += 1;
    }
}

fn is_port_ok(port: u16) -> bool {
    let address = format!("127.0.0.1:{}", port);
    match TcpStream::connect(&address) {
        Ok(_) => {
            println!("[{}] Port {} is busy 🚧", "INFO".green(), port);
            false
        }
        Err(_) => match TcpListener::bind(&address) {
            Ok(_) => true,
            Err(_) => false,
        },
    }
}
