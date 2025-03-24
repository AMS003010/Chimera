use std::net::{TcpListener, TcpStream};

pub fn find_available_port(port: u16) -> u16 {
    let mut start_port = port;
    println!("🪨  Trying to bind to port {}", port);
    loop {
        if is_port_ok(start_port) {
            println!("🔱 Port {} is available", port);
            return port;
        }
        start_port += 1;
    }
}

fn is_port_ok(port: u16) -> bool {
    let address = format!("127.0.0.1:{}",port);
    match TcpStream::connect(&address) {
        Ok(_) => {
            println!("🪨  Port {} is busy 🚧", port);
            false
        }
        Err(_) => {
            match TcpListener::bind(&address) {
                Ok(_) => true,
                Err(_) => false
            }
        }
    }
}