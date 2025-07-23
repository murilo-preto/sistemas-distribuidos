use std::io::{self, Read, Write};
use std::net::TcpStream;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::SystemTime;

#[derive(Debug)]
pub struct Message {
    pub command: String,
    pub key: String,
    pub value: String,
    pub timestamp: SystemTime,
}

pub fn connect_to_server(address: &str) -> io::Result<TcpStream> {
    let stream = TcpStream::connect(address)?;
    println!("Connected to {address}");
    Ok(stream)
}

pub fn send_command(
    stream: &Arc<Mutex<Option<TcpStream>>>,
    command: &str,
    is_connected: &Arc<AtomicBool>,
) -> std::io::Result<()> {
    if let Some(stream) = &mut *stream.lock().unwrap() {
        println!("Sending: {command}");
        stream.write_all(command.as_bytes())?;

        let mut buffer = [0; 1024];
        match stream.read(&mut buffer) {
            Ok(bytes_read) if bytes_read > 0 => {
                println!(
                    "Server response: {}",
                    String::from_utf8_lossy(&buffer[0..bytes_read])
                );
                Ok(())
            }
            Ok(_) => {
                println!("No response from server");
                Ok(())
            }
            Err(e) => {
                is_connected.store(false, Ordering::SeqCst);
                Err(e)
            }
        }
    } else {
        println!("Connection lost");
        is_connected.store(false, Ordering::SeqCst);
        Err(io::Error::new(
            io::ErrorKind::NotConnected,
            "Not connected to server",
        ))
    }
}

// ----------------------------------

pub fn add(left: u64, right: u64) -> u64 {
    left + right
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {
        let result = add(2, 2);
        assert_eq!(result, 4);
    }
}
