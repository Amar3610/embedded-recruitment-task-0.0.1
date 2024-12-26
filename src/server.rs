use crate::message::{client_message, ServerMessage};
use crate::message::{server_message, AddResponse, ClientMessage};
use prost::Message;
use std::io::{ErrorKind, Read, Write};
use std::time::Duration;
use std::{
    io::{self},
    net::{TcpListener, TcpStream},
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc,
    },
    thread,
};

struct Client {
    stream: TcpStream,
}
impl Client {
    pub fn new(stream: TcpStream) -> Self {
        Client { stream }
    }

    pub fn handle(&mut self) -> io::Result<()> {
        let mut buffer = [0; 512];
        // Read data from the client
        let bytes_read = self.stream.read(&mut buffer)?;
        if bytes_read == 0 {
            println!("Client disconnected.");
            return Ok(());
        }

        let request = ClientMessage::decode(&buffer[..bytes_read])?;

        let message = match request.message.unwrap() {
            client_message::Message::EchoMessage(msg) => {
                println!("Received EchoMessage: {}", msg.content);
                // Return ServerMessage with EchoMessage
                ServerMessage {
                    message: Some(server_message::Message::EchoMessage(msg)),
                }
            }
            client_message::Message::AddRequest(req) => {
                println!("Received AddRequest: {} + {}", req.a, req.b);
                let result = req.a + req.b;
                // Return ServerMessage with AddResponse
                ServerMessage {
                    message: Some(server_message::Message::AddResponse(AddResponse { result })),
                }
            }
        };

        let response = message.encode_to_vec();
        self.stream.write_all(&response)?;
        self.stream.flush()?;

        Ok(())
    }
}

pub struct Server {
    listener: TcpListener,
    is_running: Arc<AtomicBool>,
}

impl Server {
    /// Creates a new server instance
    pub fn new(addr: &str) -> io::Result<Self> {
        let listener = TcpListener::bind(addr)?;
        let is_running = Arc::new(AtomicBool::new(false));
        Ok(Server {
            listener,
            is_running,
        })
    }

    /// Runs the server, listening for incoming connections and handling them
    pub fn run(&self) -> io::Result<()> {
        self.is_running.store(true, Ordering::Release); // Set the server as running
        self.listener.set_nonblocking(true)?;
        println!("Server is running on {}", self.listener.local_addr()?);

        while self.is_running.load(Ordering::Acquire) {
            match self.listener.accept() {
                Ok((stream, addr)) => {
                    println!("New connection: {}", addr);
                    let is_running = Arc::clone(&self.is_running);
                    let mut client = Client::new(stream);
                    thread::spawn(move || {
                        while is_running.load(Ordering::Acquire) {
                            if let Err(e) = client.handle() {
                                match e.kind() {
                                    ErrorKind::WouldBlock => {
                                        thread::sleep(Duration::from_millis(100));
                                    }
                                    _ => {
                                        eprintln!("Server error: {}", e);
                                    }
                                }
                            }
                        }
                    });
                }
                Err(ref e) if e.kind() == ErrorKind::WouldBlock => {
                    // No incoming connections, sleep briefly to reduce CPU usage
                    thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("accept error: {}", e);
                }
            }
        }

        println!("Server stopped.");
        Ok(())
    }

    /// Stops the server by setting the `is_running` flag to `false`
    pub fn stop(&self) {
        if self.is_running.load(Ordering::Acquire) {
            self.is_running.store(false, Ordering::Release);
            println!("Shutdown signal sent.");
        } else {
            eprintln!("Server was already stopped or not running.");
        }
    }
}
