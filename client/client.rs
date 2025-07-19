use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::Duration;

fn connect_to_server(address: &str) -> std::io::Result<TcpStream> {
    let stream = TcpStream::connect(address)?;
    println!("Connected to {}", address);
    Ok(stream)
}

fn get_input(prompt: &str) -> String {
    println!("{}", prompt);

    let mut input = String::new();
    match io::stdin().read_line(&mut input) {
        Ok(_) => input.trim().to_string(),
        Err(e) => {
            println!("Error reading input: {}", e);
            String::new()
        }
    }
}

fn handle_server_connection(
    server_addr: &str,
    running: Arc<AtomicBool>,
    is_connected: Arc<AtomicBool>,
) -> std::io::Result<()> {
    match connect_to_server(server_addr) {
        Ok(mut stream) => {
            is_connected.store(true, Ordering::SeqCst);
            let message = "Hello from client!";
            match stream.write_all(message.as_bytes()) {
                Ok(_) => {
                    println!("Sent initial message: '{}'", message);

                    let mut buffer = [0; 1024];
                    match stream.read(&mut buffer) {
                        Ok(bytes_read) if bytes_read > 0 => {
                            println!(
                                "Initial response: {}",
                                String::from_utf8_lossy(&buffer[0..bytes_read])
                            );
                            println!("Connected successfully!");
                            Ok(())
                        }
                        Ok(_) => {
                            println!("Connected to {} but received no response", server_addr);
                            Ok(())
                        }
                        Err(e) => {
                            is_connected.store(false, Ordering::SeqCst);
                            println!("Error reading from {}: {}", server_addr, e);
                            Err(e)
                        }
                    }
                }
                Err(e) => {
                    is_connected.store(false, Ordering::SeqCst);
                    println!("Error sending to {}: {}", server_addr, e);
                    Err(e)
                }
            }
        }
        Err(e) => {
            is_connected.store(false, Ordering::SeqCst);
            println!("Failed to connect to {}: {}", server_addr, e);
            Err(e)
        }
    }
}

fn main() -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let is_connected = Arc::new(AtomicBool::new(false));

    // Main command loop
    println!(
        "---\nType:\nINIT - Connect to server\nPUT - To write to database\nGET - To retrieve from database\nEXIT - To quit\n---\n"
    );
    loop {
        let action = get_input("").to_lowercase();

        match action.as_str() {
            "init" => {
                if is_connected.load(Ordering::SeqCst) {
                    println!("Already connected to a server");
                } else if running.load(Ordering::SeqCst) {
                    let servers = ["127.0.0.1:10097", "127.0.0.1:10098", "127.0.0.1:10099"];
                    let running_clone = running.clone();
                    let is_connected_clone = is_connected.clone();

                    thread::spawn(move || {
                        for server_addr in &servers {
                            if let Ok(_) = handle_server_connection(
                                server_addr,
                                running_clone.clone(),
                                is_connected_clone.clone(),
                            ) {
                                break;
                            }
                        }
                    });
                } else {
                    println!("Shutdown in progress, cannot initialize new connection");
                }
            }
            "put" => {
                if is_connected.load(Ordering::SeqCst) {
                    println!("PUT operation selected");
                    // Add your PUT logic here
                } else {
                    println!("Not connected to any server. Use INIT first");
                }
            }
            "get" => {
                if is_connected.load(Ordering::SeqCst) {
                    println!("GET operation selected");
                    // Add your GET logic here
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
