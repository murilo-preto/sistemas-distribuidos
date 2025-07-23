use std::collections::HashMap;
use std::io::{self, BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::AtomicBool;
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

use shared::Message;
use shared::send_msg;

struct ServerState {
    db: Arc<Mutex<HashMap<String, String>>>,
    is_leader: bool,
    leader_port: u16,
    self_port: u16,
    followers: Arc<Mutex<Vec<u16>>>,
    server_timestamp: SystemTime,
}

impl ServerState {
    fn new(port: u16, leader_port: u16) -> Self {
        let is_leader = port == leader_port;
        let self_port = port;
        let server_timestamp = SystemTime::now();
        ServerState {
            db: Arc::new(Mutex::new(HashMap::new())),
            is_leader,
            leader_port,
            self_port,
            followers: Arc::new(Mutex::new(Vec::new())),
            server_timestamp,
        }
    }

    fn register_follower(&self, port: u16) {
        let mut followers = self.followers.lock().unwrap();
        if !followers.contains(&port) {
            followers.push(port);
            println!("[Leader] Registered new follower on port {port}\n");
        }
    }
}

fn handle_client(mut stream: TcpStream, state: Arc<ServerState>) {
    let port = state.self_port;
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
                    ["put", key, value] => on_put(key, value, Arc::clone(&state)),
                    ["get", key] => on_get(key, Arc::clone(&state)),
                    ["register", port_str] => on_register(port_str, Arc::clone(&state)),
                    ["replicate", key, value] => on_replicate(key, value, Arc::clone(&state)),
                    ["put", ..] => "ERR: PUT requires 2 arguments\n".to_string(),
                    ["get", ..] => "ERR: GET requires 1 argument\n".to_string(),
                    ["new_connection", ..] => "New client connection received\n".to_string(),
                    _ => "ERR: Unknown command\n".to_string(),
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

fn handle_client_serialized(mut stream: TcpStream, state: Arc<ServerState>) {
    let port = state.self_port;
    println!("[Port {port}] Client connected");
    let mut buffer = [0; 1024];

    loop {
        match stream.read(&mut buffer) {
            Ok(0) => {
                println!("[Port {port}] Client disconnected");
                break;
            }
            Ok(n) => {
                let message_str = String::from_utf8_lossy(&buffer[0..n]);
                println!("[Port {}] Received raw: {}", port, message_str.trim());

                let parsed: Result<Message, _> = serde_json::from_str(&message_str);
                let response = match parsed {
                    Ok(msg) => {
                        println!("[Port {}] Parsed Message: {:?}", port, msg);
                        match msg.command.as_str() {
                            "put" => on_put(&msg.key, &msg.value, Arc::clone(&state)),
                            "get" => on_get(&msg.key, Arc::clone(&state)),
                            "register" => on_register(&msg.value, Arc::clone(&state)),
                            "replicate" => on_replicate(&msg.key, &msg.value, Arc::clone(&state)),
                            "new_connection" => "New client connection received\n".to_string(),
                            _ => "ERR: Unknown command\n".to_string(),
                        }
                    }
                    Err(e) => {
                        println!("[Port {}] Failed to parse message: {}", port, e);
                        "ERR: Failed to parse message\n".to_string()
                    }
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

fn on_put(key: &str, value: &str, state: Arc<ServerState>) -> String {
    if state.is_leader {
        // Update leader's database
        let mut db = state.db.lock().unwrap();
        db.insert(key.to_string(), value.to_string());
        println!("[Leader] Added key: {key}");

        // Replicate to followers
        let followers = state.followers.lock().unwrap().clone();
        for follower_port in followers {
            if let Err(e) = replicate_to_follower(key, value, follower_port) {
                println!("[Leader] Failed to replicate to {follower_port}: {e}");
            }
        }

        format!("OK: Inserted '{key}'='{value}'\n")
    } else {
        // Forward to leader
        let leader_addr = format!("127.0.0.1:{}", state.leader_port);
        match TcpStream::connect(leader_addr) {
            Ok(mut stream) => {
                let is_connected = Arc::new(AtomicBool::new(true));
                let cmd = format!("put {key} {value}\n");

                let msg = Message {
                    command: "put".to_string(),
                    key: key.to_string(),
                    value: value.to_string(),
                    timestamp: SystemTime::now(),
                };

                let stream = Arc::new(Mutex::new(Some(stream)));
                if let Err(e) = send_msg(&stream, &msg, &is_connected) {
                    format!("Error sending PUT command: {e}")
                } else {
                    format!("Unhandled")
                }
            }
            Err(e) => {
                println!("Failed to connect to leader: {e}");
                format!("ERR: Not leader and failed to connect to leader: {e}\n")
            }
        }
    }
}

fn on_get(key: &str, state: Arc<ServerState>) -> String {
    let db = state.db.lock().unwrap();
    match db.get(key) {
        Some(value) => format!("OK: {key}={value}\n"),
        None => format!("ERR: Key '{key}' not found\n"),
    }
}

fn on_register(port_str: &str, state: Arc<ServerState>) -> String {
    if !state.is_leader {
        return "ERR: Only leader can register followers\n".to_string();
    }

    match port_str.parse::<u16>() {
        Ok(port) => {
            state.register_follower(port);
            format!("OK: Registered follower on port {port}\n")
        }
        Err(_) => "ERR: Invalid port number\n".to_string(),
    }
}

fn on_replicate(key: &str, value: &str, state: Arc<ServerState>) -> String {
    if state.is_leader {
        return "ERR: Leader should not receive replicate commands\n".to_string();
    }

    println!("[Follower] Replicating: {key} = {value}");
    let mut db = state.db.lock().unwrap();
    db.insert(key.to_string(), value.to_string());
    format!("OK: Replicated '{key}'='{value}'\n")
}

fn replicate_to_follower(key: &str, value: &str, port: u16) -> io::Result<()> {
    let addr = format!("127.0.0.1:{port}");
    let mut stream = TcpStream::connect(addr)?;
    let cmd = format!("replicate {key} {value}\n");
    stream.write_all(cmd.as_bytes())?;

    let mut reader = BufReader::new(&stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;

    if response.starts_with("OK:") {
        Ok(())
    } else {
        Err(io::Error::new(io::ErrorKind::Other, response))
    }
}

fn start_server(port: u16, leader_port: u16) -> io::Result<()> {
    let listener = TcpListener::bind(format!("127.0.0.1:{port}"))?;
    println!("Server listening on 127.0.0.1:{port}");

    let state = Arc::new(ServerState::new(port, leader_port));

    // Register with leader if this is a follower
    if !state.is_leader {
        register_with_leader(port, leader_port)?;
    }

    for stream in listener.incoming() {
        let state_clone = Arc::clone(&state);
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client_serialized(stream, state_clone);
                });
            }
            Err(e) => println!("[Port {port}] Error: {e}"),
        }
    }
    Ok(())
}

fn register_with_leader(follower_port: u16, leader_port: u16) -> io::Result<()> {
    let addr = format!("127.0.0.1:{leader_port}");

    let stream = TcpStream::connect(addr)?;
    let stream_clone = stream.try_clone()?;
    let protected_stream = Arc::new(Mutex::new(Some(stream_clone)));

    let is_connected = Arc::new(AtomicBool::new(true));

    let msg = Message {
        command: "register".to_string(),
        key: "".to_string(),
        value: follower_port.to_string(),
        timestamp: SystemTime::now(),
    };

    if let Err(e) = send_msg(&protected_stream, &msg, &is_connected) {
        println!("Error sending PUT command: {e}");
        Ok(())
    } else {
        Ok(())
    }
}

fn main() -> io::Result<()> {
    print!("Enter port number: ");
    io::stdout().flush()?;

    let mut port_input = String::new();
    io::stdin().lock().read_line(&mut port_input)?;
    let port: u16 = port_input
        .trim()
        .parse()
        .map_err(|_| io::Error::new(io::ErrorKind::InvalidInput, "Invalid port number"))?;

    const LEADER_PORT: u16 = 10097;

    println!("Starting server on port {port}...");
    start_server(port, LEADER_PORT)
}
