use embedded_recruitment_task::message::{client_message, EchoMessage, ServerMessage};
use embedded_recruitment_task::server::Server;
use log::info;
use prost::Message;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpStream, ToSocketAddrs};
use std::sync::Arc;
use std::thread;
use std::thread::{sleep, JoinHandle};

fn setup_server_thread(server: Arc<Server>) -> JoinHandle<()> {
    thread::spawn(move || {
        server.run().expect("Server encountered an error");
    })
}

fn create_server() -> Arc<Server> {
    Arc::new(Server::new("localhost:8080").expect("Failed to start server"))
}

fn main() {
    let server = create_server();
    let server_handle = setup_server_thread(server.clone());

    let address = "localhost:8080".to_string();

    // Handle the conversion and potential error
    let socket_addrs: Vec<SocketAddr> = match address.to_socket_addrs() {
        Ok(addrs) => addrs.collect(),
        Err(e) => {
            eprintln!("Failed to resolve address: {}", e);
            return;
        }
    };

    // Try to connect to the first address with a timeout
    let mut stream = match TcpStream::connect_timeout(
        &socket_addrs[0],
        std::time::Duration::from_millis(1000),
    ) {
        Ok(stream) => {
            println!("Connected to {}", address);
            stream
        }
        Err(e) => {
            eprintln!("Failed to connect: {}", e);
            return;
        }
    };

    let mut x: i32 = 2;

    while x > 0 {
        x = x - 1;

        // Prepare multiple messages
        let messages = vec!["Hello, World!".to_string()];

        let mut echo_message = EchoMessage::default();
        echo_message.content = messages[0].clone();
        let message = client_message::Message::EchoMessage(echo_message);

        let mut buffer = Vec::new();
        message.encode(&mut buffer);

        // Send the buffer to the server
        stream.write_all(&buffer).expect("Failed to send message");
        stream.flush().expect("Failed to flush stream");

        println!("Receiving message from the server");
        let mut buffer = vec![0u8; 1024];
        let bytes_read = stream.read(&mut buffer).expect("Failed to receive message");
        if bytes_read == 0 {
            info!("Server disconnected.");
            continue;
        }

        println!("Received {} bytes from the server", bytes_read);

        // Decode the received message
        let server_message =
            ServerMessage::decode(&buffer[..bytes_read]).expect("Failed to decode server message");
        println!("server_message {:?}", server_message);
        println!();
        sleep(std::time::Duration::from_millis(1000));
    }

    // Stop the server and wait for thread to finish
    server.stop();
    server_handle.join().unwrap();
}
