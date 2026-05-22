use std::sync::Arc;
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use crate::domain::traits::DiskManager;
use crate::ipc::protocol::{IsekaiCommand, IsekaiEvent};

pub struct IpcServer {
    disk_manager: Arc<dyn DiskManager>,
    port: u16
}

impl IpcServer {
    pub fn new(disk_manager: Arc<dyn DiskManager>, port: u16) -> Self {
        Self { disk_manager, port }
    }

    pub async fn start(&self) -> Result<(), Box<dyn std::error::Error>> {
        let address = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&address).await?;
        println!("IPC Server listening on {}", address);

        loop {
            let (mut socket, _) = listener.accept().await?;
            println!("Client connected to IPC socket!");

            let manager = Arc::clone(&self.disk_manager);
            tokio::spawn(async move {
                let mut buf = [0; 1024];

                loop {
                    let n = match socket.read(&mut buf).await {
                        Ok(n) if (n) == 0 => {
                            println!("Client disconnected from IPC socket!");
                            return;
                        }
                        Ok(n) => n,
                        Err(e) => {
                            eprintln!("Socket read error: {}", e);
                            return;
                        }
                    };
                    let received = String::from_utf8_lossy(&buf[..n]);

                    if let Ok(command) = serde_json::from_str::<IsekaiCommand>(received.trim()) {
                        match command {
                            IsekaiCommand::GetDisks => {
                                let event = match manager.get_disks().await {
                                    Ok(disks) => IsekaiEvent::DisksLoaded { disks },
                                    Err(e) => IsekaiEvent::FatalError {
                                        message: format!("Failed to read disks: {}", e)
                                    }
                                };
                                Self::send_event(&mut socket, &event).await;
                            }
                            IsekaiCommand::ShrinkPartition { disk_num, size_gb } => {
                                let event = IsekaiEvent::Progress {
                                    step: format!("Requested shrink on disk {} for {}GB", disk_num, size_gb),
                                    percent: 0
                                };
                                Self::send_event(&mut socket, &event).await;
                            }
                        }
                    } else {
                        eprintln!("Failed to parse JSON: {}", received);
                    }
                }
            });
        }
    }

    async fn send_event(socket: &mut TcpStream, event: &IsekaiEvent) {
        if let Ok(mut json) = serde_json::to_string(&event) {
            json.push('\n');
            if let Err(e) = socket.write_all(json.as_bytes()).await {
                eprintln!("Failed to send event: {}", e)
            }
        }
    }
}