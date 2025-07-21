use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn handle_client(mut stream: TcpStream, port: u16, db: Arc<Mutex<HashMap<String, String>>>) {
    println!("[Port {}] Client connected", port);
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("[Port {}] Client disconnected", port);
                break;
            }
            Ok(n) => {
                let message = String::from_utf8_lossy(&buffer[0..n]);
                println!("[Port {}] Received: {}", port, message.trim());

                let input = message.trim();
                let parts: Vec<&str> = input.split_whitespace().collect();

                let response = match parts.as_slice() {
                    ["put", key, value] => {
                        on_put(key.to_string(), value.to_string(), port, Arc::clone(&db))
                    },
                    ["get", key] => {
                        on_get(key.to_string(), Arc::clone(&db))
                    },
                    ["put", ..] => "ERR: PUT requires 2 arguments".to_string(),
                    ["get", ..] => "ERR: GET requires 1 argument".to_string(),
                    _ => "ERR: Unknown command (valid: PUT <key> <value> | GET <key>)".to_string(),
                };

                // Send only one response
                if stream.write_all(response.as_bytes()).is_err() {
                    break;
                }
            }
            Err(e) => {
                println!("[Port {}] Error reading from client: {}", port, e);
                break;
            }
        }
    }
}

fn start_server(port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    println!("Server listening on 127.0.0.1:{}", port);

    let database: HashMap<String, String> = HashMap::new();
    let shared_db = Arc::new(Mutex::new(database));

    for stream in listener.incoming() {
        let db_clone = Arc::clone(&shared_db);
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream, port, db_clone);
                });
            }
            Err(e) => println!("[Port {}] Error: {}", port, e),
        }
    }
    Ok(())
}

fn on_put(key: String, value: String, port: u16, db: Arc<Mutex<HashMap<String, String>>>) -> String {
    if port == 10097 {
        let mut db = db.lock().unwrap();
        db.insert(key.clone(), value.clone());
        println!("[Leader] Added key: {}", key);
        format!("OK: Inserted '{}'='{}'", key, value)
    } else {
        format!("ERR: Server on port {} is not leader", port)
    }
}

fn on_get(key: String, db: Arc<Mutex<HashMap<String, String>>>) -> String {
    let db = db.lock().unwrap();
    match db.get(&key) {
        Some(value) => format!("OK: {}={}", key, value),
        None => format!("ERR: Key '{}' not found", key),
    }
}

fn main() -> std::io::Result<()> {
    print!("Enter port number: ");
    io::stdout().flush()?;

    let stdin = io::stdin();
    let mut line = String::new();
    stdin.lock().read_line(&mut line)?;

    let port: u16 = match line.trim().parse() {
        Ok(port) => port,
        Err(_) => {
            println!("Invalid port number. Please enter a number between 1 and 65535.");
            return Ok(());
        }
    };

    println!("Starting server on port {}...", port);
    start_server(port)
}
