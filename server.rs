use std::net::{TcpListener, TcpStream};
use std::io::{self, Read, Write, BufRead};
use std::thread;

fn handle_client(mut stream: TcpStream, port: u16) {
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
                
                // Send response
                let response = format!("Echo from server on port {}: {}", port, message);
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
    
    for stream in listener.incoming() {
        match stream {
            Ok(stream) => {
                thread::spawn(move || {
                    handle_client(stream, port);
                });
            }
            Err(e) => println!("[Port {}] Error: {}", port, e),
        }
    }
    Ok(())
}

fn main() -> std::io::Result<()> {
    print!("Enter port number: ");
    io::stdout().flush()?; // Ensure the prompt is displayed
    
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
    
    if let Err(e) = start_server(port) {
        println!("Failed to start server: {}", e);
    }
    
    Ok(())
}
