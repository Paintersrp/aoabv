use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;

fn handle_connection(mut stream: TcpStream) {
    let mut buffer = [0_u8; 512];
    let _ = stream.read(&mut buffer);
    let response = b"HTTP/1.1 200 OK\r\nContent-Length: 5\r\n\r\nhello";
    let _ = stream.write_all(response);
    let _ = stream.flush();
}

fn main() -> std::io::Result<()> {
    let addr: SocketAddr = "127.0.0.1:3000".parse().expect("valid socket address");

    tracing::info!(%addr, "starting simd");

    let listener = TcpListener::bind(addr)?;

    for stream in listener.incoming().take(1) {
        match stream {
            Ok(stream) => {
                thread::spawn(|| handle_connection(stream));
            }
            Err(err) => eprintln!("listener error: {err}"),
        }
    }

    Ok(())
}
