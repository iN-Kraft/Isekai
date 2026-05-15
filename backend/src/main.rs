use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::TcpListener;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Isekai Daemon starting...");

    let listener = TcpListener::bind("127.0.0.1:45454").await?;
    println!("Listening on {}", listener.local_addr()?);

    loop {
        let (mut socket, _) = listener.accept().await?;
        println!("Connection received");

        tokio::spawn(async move {
            let mut buf = [0; 1024];

            loop {
                let n = match socket.read(&mut buf).await {
                    Ok(n) if n == 0 => return,
                    Ok(n) => n,
                    Err(e) => {
                        eprintln!("Failed to read from socket; err = {:?}", e);
                        return;
                    }
                };

                let received = String::from_utf8_lossy(&buf[0..n]);
                println!("Received: {}", received);

                let response = format!("{{\"status\": \"Rust successfully received: {}\"}}\n", received.trim());
                if let Err(e) = socket.write_all(response.as_bytes()).await {
                    eprintln!("Failed to write to socket; err = {:?}", e);
                    return;
                }
            }
        });
    }
}
