use rand::rng;
use rand::seq::SliceRandom;
use std::env;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
//use std::time::SystemTime;

use shared::{Message, connect_to_server, secs_since_epoch, send_msg};

fn get_input(prompt: &str) -> String {
    println!("{prompt}");
    let mut input = String::new();
    io::stdin()
        .read_line(&mut input)
        .expect("Failed to read input");
    input.trim().to_string()
}

fn handle_server_connection(
    server_addr: &str,
    is_connected: Arc<AtomicBool>,
    stream: Arc<Mutex<Option<TcpStream>>>,
) -> std::io::Result<()> {
    match connect_to_server(server_addr) {
        Ok(mut s) => {
            let msg = Message {
                command: "new_connection".to_string(),
                key: "".to_string(),
                value: "".to_string(),
                timestamp: secs_since_epoch(),
            };

            let serialized = serde_json::to_string(&msg)?;
            s.write_all(serialized.as_bytes())?;

            let mut buffer = [0; 1024];
            match s.read(&mut buffer) {
                Ok(bytes_read) if bytes_read > 0 => {
                    /*
                    println!(
                        "Initial response: {}",
                        String::from_utf8_lossy(&buffer[..bytes_read])
                    );
                    */
                }
                Ok(_) => {
                    println!("Connected to {} but received no response", server_addr);
                }
                Err(e) => {
                    println!("Error reading from {}: {}", server_addr, e);
                    return Err(e);
                }
            }

            *stream.lock().unwrap() = Some(s);
            is_connected.store(true, Ordering::SeqCst);
            println!("Connected successfully to server");
            Ok(())
        }
        Err(e) => {
            println!("Failed to connect to {}: {}", server_addr, e);
            is_connected.store(false, Ordering::SeqCst);
            *stream.lock().unwrap() = None;
            Err(e)
        }
    }
}

fn main() -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let is_connected = Arc::new(AtomicBool::new(false));
    let stream = Arc::new(Mutex::new(None::<TcpStream>));

    let args: Vec<String> = env::args().skip(1).collect();

    let servers: Vec<String> = args
        .iter()
        .map(|port| format!("127.0.0.1:{}", port))
        .collect();

    println!("Server list: {:?}", servers);

    println!(
        "---\nType:\nINIT - Connect to server\nPUT - To write to database\nGET - To retrieve from database\nEXIT - To quit\n---\n"
    );

    loop {
        let action = get_input("Enter command:").to_lowercase();

        match action.as_str() {
            "init" => {
                if is_connected.load(Ordering::SeqCst) {
                    println!("Already connected to a server");
                } else {
                    //let mut servers = vec!["127.0.0.1:10097", "127.0.0.1:10098", "127.0.0.1:10099"];
                    let mut servers = servers.clone();
                    let is_connected_clone = is_connected.clone();
                    let stream_clone = stream.clone();

                    thread::spawn(move || {
                        let mut rng = rng();
                        servers.shuffle(&mut rng);

                        for server_addr in &servers {
                            if handle_server_connection(
                                server_addr,
                                is_connected_clone.clone(),
                                stream_clone.clone(),
                            )
                            .is_ok()
                            {
                                break;
                            }
                        }
                    });
                }
            }
            "put" => {
                if is_connected.load(Ordering::SeqCst) {
                    let key = get_input("Enter key:");
                    let value = get_input("Enter value:");

                    let msg = Message {
                        command: "put".to_string(),
                        key,
                        value,
                        timestamp: secs_since_epoch(),
                    };

                    if let Err(e) = send_msg(&stream, &msg, &is_connected) {
                        println!("Error sending PUT command: {e}");
                    }
                } else {
                    println!("Not connected to any server. Use INIT first");
                }
            }
            "get" => {
                if is_connected.load(Ordering::SeqCst) {
                    let key = get_input("Enter key:");

                    let msg = Message {
                        command: "get".to_string(),
                        key,
                        value: "".to_string(),
                        timestamp: secs_since_epoch(),
                    };

                    if let Err(e) = send_msg(&stream, &msg, &is_connected) {
                        println!("Error sending GET command: {e}");
                    }
                } else {
                    println!("Not connected to any server. Use INIT first");
                }
            }
            "exit" => {
                println!("Exiting...");
                running.store(false, Ordering::SeqCst);
                is_connected.store(false, Ordering::SeqCst);
                break;
            }
            _ => println!("Unknown command"),
        }
    }

    Ok(())
}
