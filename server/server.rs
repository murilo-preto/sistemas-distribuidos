use crate::io::BufReader;
use std::collections::HashMap;
use std::io::{self, BufRead, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex, atomic::AtomicBool};
use std::thread;

fn handle_client(mut stream: TcpStream, port: u16, db: Arc<Mutex<HashMap<String, String>>>) {
    println!("[Port {port}] Client connected");
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("[Port {port}] Client disconnected");
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
                    }
                    ["get", key] => on_get(key.to_string(), Arc::clone(&db)),
                    ["put", ..] => "ERR: PUT requires 2 arguments".to_string(),
                    ["get", ..] => "ERR: GET requires 1 argument".to_string(),
                    ["new_connection"] => "Client requested new connetion".to_string(),
                    _ => "ERR: Unknown command (valid: PUT <key> <value> | GET <key>)".to_string(),
                };

                if stream.write_all(response.as_bytes()).is_err() {
                    break;
                }
            }
            Err(e) => {
                println!("[Port {port}] Error reading from client: {e}");
                break;
            }
        }
    }
}

fn start_server(port: u16) -> std::io::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
    println!("Server listening on 127.0.0.1:{port}");

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
            Err(e) => println!("[Port {port}] Error: {e}"),
        }
    }
    Ok(())
}

fn on_put(
    key: String,
    value: String,
    port: u16,
    db: Arc<Mutex<HashMap<String, String>>>,
) -> String {
    if port == 10097 {
        // Leader handling
        let mut db = db.lock().unwrap();
        db.insert(key.clone(), value.clone());
        println!("[Leader] Added key: {key}");
        format!("OK: Inserted '{key}'='{value}'")
    } else {
        // Non-leader handling
        let leader_addr = "127.0.0.1:10097";

        // Attempt to connect to leader
        match TcpStream::connect(leader_addr) {
            Ok(mut stream) => {
                println!("Handed put request over to leader");
                let command = format!("put {} {}\n", key, value);

                // Send command to leader
                if let Err(e) = stream.write_all(command.as_bytes()) {
                    return format!("ERR: Failed to send command to leader: {}", e);
                }

                // Read leader's response
                let mut reader = BufReader::new(&stream);
                let mut response = String::new();
                match reader.read_line(&mut response) {
                    Ok(_) => response,
                    Err(e) => format!("ERR: Failed to read leader response: {}", e),
                }
            }
            Err(e) => {
                // Connection failed
                println!("Failed to connect to leader: {}", e);
                format!("ERR: Not leader and failed to connect to leader: {}", e)
            }
        }
    }
}

fn on_get(key: String, db: Arc<Mutex<HashMap<String, String>>>) -> String {
    let db = db.lock().unwrap();
    match db.get(&key) {
        Some(value) => format!("OK: {key}={value}"),
        None => format!("ERR: Key '{key}' not found"),
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

    println!("Starting server on port {port}...");
    start_server(port)
}
