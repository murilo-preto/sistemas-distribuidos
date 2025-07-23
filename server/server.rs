use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::{Arc, Mutex};
use std::thread;

fn handle_client(mut client_stream: TcpStream, port: u16, db: Arc<Mutex<HashMap<String, String>>>) {
    println!("[Port {port}] Client connected");
    let mut buffer = [0; 1024];

    loop {
        match client_stream.read(&mut buffer) {
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
                    ["put", ..] => "ERR: PUT requires 2 arguments\n".to_string(),
                    ["get", ..] => "ERR: GET requires 1 argument\n".to_string(),
                    ["new_connection"] => "Client requested new connection\n".to_string(),
                    _ => {
                        "ERR: Unknown command (valid: PUT <key> <value> | GET <key>)\n".to_string()
                    }
                };

                if client_stream.write_all(response.as_bytes()).is_err() {
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

    let shared_db = Arc::new(Mutex::new(HashMap::new()));

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
        format!("OK: Inserted '{key}'='{value}'\n")
    } else {
        // Non-leader handling - create transient connection to leader
        let leader_addr = "127.0.0.1:10097";
        match TcpStream::connect(leader_addr) {
            Ok(mut leader_stream) => {
                println!("[Port {port}] Handed PUT to leader");
                let command = format!("put {} {}\n", key, value);

                // Send command to leader
                if let Err(e) = leader_stream.write_all(command.as_bytes()) {
                    return format!("ERR: Failed to send to leader: {}\n", e);
                }

                // Read leader's response
                let mut reader = BufReader::new(&leader_stream);
                let mut response = String::new();
                match reader.read_line(&mut response) {
                    Ok(_) => response, // Includes trailing \n
                    Err(e) => format!("ERR: Failed reading leader response: {}\n", e),
                }
            }
            Err(e) => {
                println!("[Port {port}] Leader connection failed: {e}");
                format!("ERR: Leader connection failed: {}\n", e)
            }
        }
    }
}

fn on_get(key: String, db: Arc<Mutex<HashMap<String, String>>>) -> String {
    let db = db.lock().unwrap();
    match db.get(&key) {
        Some(value) => format!("OK: {key}={value}\n"),
        None => format!("ERR: Key '{key}' not found\n"),
    }
}

fn main() -> std::io::Result<()> {
    print!("Enter port number: ");
    io::stdout().flush()?;

    let mut line = String::new();
    io::stdin().lock().read_line(&mut line)?;

    let port: u16 = line
        .trim()
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid port number"))?;

    println!("Starting server on port {port}...");
    start_server(port)
}
