use rand::rng;
use rand::seq::SliceRandom;
use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread;
use std::time::SystemTime;

use shared::{Message, connect_to_server, send_msg};

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
            is_connected.store(true, Ordering::SeqCst);
            let message = "new_connection";
            match s.write_all(message.as_bytes()) {
                Ok(_) => {
                    println!("Sent initial message: '{message}'");
                    *stream.lock().unwrap() = Some(s);

                    let mut buffer = [0; 1024];
                    match stream.lock().unwrap().as_mut().unwrap().read(&mut buffer) {
                        Ok(bytes_read) if bytes_read > 0 => {
                            println!(
                                "Initial response: {}",
                                String::from_utf8_lossy(&buffer[0..bytes_read])
                            );
                            println!("Connected successfully!");
                            Ok(())
                        }
                        Ok(_) => {
                            println!("Connected to {server_addr} but received no response");
                            Ok(())
                        }
                        Err(e) => {
                            is_connected.store(false, Ordering::SeqCst);
                            *stream.lock().unwrap() = None;
                            println!("Error reading from {server_addr}: {e}");
                            Err(e)
                        }
                    }
                }
                Err(e) => {
                    is_connected.store(false, Ordering::SeqCst);
                    *stream.lock().unwrap() = None;
                    println!("Error sending to {server_addr}: {e}");
                    Err(e)
                }
            }
        }
        Err(e) => {
            is_connected.store(false, Ordering::SeqCst);
            *stream.lock().unwrap() = None;
            println!("Failed to connect to {server_addr}: {e}");
            Err(e)
        }
    }
}

fn main() -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let is_connected = Arc::new(AtomicBool::new(false));
    let stream = Arc::new(Mutex::new(None::<TcpStream>));

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
                    let mut servers = vec!["127.0.0.1:10097", "127.0.0.1:10098", "127.0.0.1:10099"];
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
                        timestamp: SystemTime::now(),
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
                        timestamp: SystemTime::now(),
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
