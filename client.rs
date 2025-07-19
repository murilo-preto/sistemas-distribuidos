use std::net::TcpStream;
use std::io::{Read, Write};

fn connect_to_server(address: &str) -> std::io::Result<TcpStream> {
    let stream = TcpStream::connect(address)?;
    println!("Connected to {}", address);
    Ok(stream)
}

fn main() -> std::io::Result<()> {
    // Connect to your three servers
    let servers = ["127.0.0.1:10097", "127.0.0.1:10098", "127.0.0.1:10099"];

    for server_addr in &servers {
        match connect_to_server(server_addr) {
            Ok(mut stream) => {
                // Define message as string
                let message = "Hello from client!";

                // Try to send and receive data
                match stream.write_all(message.as_bytes()) {
                    Ok(_) => {
                        println!("Sending '{}' to {}", message, server_addr);

                        // Read response
                        let mut buffer = [0; 1024];
                        match stream.read(&mut buffer) {
                            Ok(bytes_read) if bytes_read > 0 => {
                                println!("Received from {}: {}", 
                                    server_addr, 
                                    String::from_utf8_lossy(&buffer[0..bytes_read])
                                );
                                println!("Successfully communicated with {}. Stopping here.", server_addr);
                                return Ok(()); // Exit successfully after first successful communication
                            }
                            Ok(_) => {
                                println!("Connected to {} but received no data", server_addr);
                                // Continue to try next server
                            }
                            Err(e) => {
                                println!("Error reading from {}: {}", server_addr, e);
                                // Continue to try next server
                            }
                        }
                    }
                    Err(e) => {
                        println!("Error sending to {}: {}", server_addr, e);
                        // Continue to try next server
                    }
                }
            }
            Err(e) => {
                println!("Failed to connect to {}: {}", server_addr, e);
                // Continue to try next server
            }
        }
    }

    println!("No servers responded successfully");
    Ok(())
}
