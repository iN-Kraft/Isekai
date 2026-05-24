use std::sync::Arc;
use crate::domain::traits::DiskManager;
use std::error::Error;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::spawn;
use tokio::sync::mpsc::channel;
use tokio_util::codec::{Framed, LinesCodec};
use crate::ipc::handler::process_request;
use crate::ipc::protocol::{IpcRequest, OutgoingMessage};

pub struct IpcServer {
    disk_manager: Arc<dyn DiskManager>,
    port: u16,
}

impl IpcServer {
    pub fn new(disk_manager: Arc<dyn DiskManager>, port: u16) -> Self {
        Self { disk_manager, port }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = TcpListener::bind(&addr).await?;
        println!("IPC Server listening on tcp://{}", addr);

        loop {
            let (socket, remote_addr) = listener.accept().await?;
            println!("Client connected: {}", remote_addr);

            let dm = self.disk_manager.clone();
            spawn(async move {
                let framed = Framed::new(socket, LinesCodec::new());
                let (mut sink, mut stream) = framed.split();
                let (tx, mut rx) = channel::<OutgoingMessage>(100);
                let writer_task = spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if sink.send(json).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(line) => {
                            if let Ok(req) = serde_json::from_str::<IpcRequest>(&line) {
                                let tx_clone = tx.clone();
                                let dm_clone = dm.clone();

                                spawn(async move {
                                    process_request(req, dm_clone, tx_clone).await;
                                });
                            } else {
                                eprintln!("Received malformed JSON: {}", line);
                            }
                        }
                        Err(e) => {
                            eprintln!("Socket read error: {}", e);
                            break;
                        }
                    }
                }

                writer_task.abort();
                println!("Client disconnected: {}", remote_addr);
            });
        }
    }
}