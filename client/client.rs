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

fn main() -> std::io::Result<()> {
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    let mut is_connected = false;

    // Set up Ctrl+C handler
    ctrlc::set_handler(move || {
        println!("\nReceived Ctrl+C signal. Shutting down...");
        r.store(false, Ordering::SeqCst);
    })
    .expect("Error setting Ctrl+C handler");

    let mut action = get_input("Type:\nINIT - For initialising the client\nPUT - To write to database\nGET - To retrieve from database\n");
    action = action.to_lowercase();
    println!("Action {}", action);

    match action.as_str() {
        "init" => println!("a"),
        "get" => println!("b"),
        "put" => println!("c"),
        _ => println!("Unknown"),
    }

    let servers = ["127.0.0.1:10097", "127.0.0.1:10098", "127.0.0.1:10099"];
    for server_addr in &servers {
        match connect_to_server(server_addr) {
            Ok(mut stream) => {
                is_connected = true;
                let message = "Hello from client!";
                match stream.write_all(message.as_bytes()) {
                    // On sucessfull conection
                    Ok(_) => {
                        println!("Sent initial message: '{}'", message);

                        // Read initial response
                        let mut buffer = [0; 1024];
                        match stream.read(&mut buffer) {
                            Ok(bytes_read) if bytes_read > 0 => {
                                println!(
                                    "Initial response: {}",
                                    String::from_utf8_lossy(&buffer[0..bytes_read])
                                );

                                println!("Connected successfully! Press Ctrl+C to disconnect.");

                                // Keep  alive until end signal
                                while running.load(Ordering::SeqCst) {
                                    thread::sleep(Duration::from_millis(100));
                                }

                                println!("Disconnecting from {}...", server_addr);
                                return Ok(());
                            }
                            Ok(_) => {
                                println!("Connected to {} but received no response", server_addr);
                            }
                            Err(e) => {
                                println!("Error reading from {}: {}", server_addr, e);
                            }
                        }
                    }
                    // On failed connection
                    Err(e) => {
                        println!("Error sending to {}: {}", server_addr, e);
                    }
                }
            }
            Err(e) => {
                println!("Failed to connect to {}: {}", server_addr, e);
            }
        }
    }

    println!("No servers responded successfully");
    Ok(())
}
