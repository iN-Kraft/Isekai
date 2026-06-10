use std::sync::{Arc, RwLock};
use crate::domain::traits::DiskManager;
use std::error::Error;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::net::windows::named_pipe::ServerOptions;
use tokio::spawn;
use tokio::sync::mpsc::channel;
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec, LinesCodec};
use crate::application::{AppContext, APP_CONTEXT};
use crate::ipc::handler::process_request;
use crate::ipc::protocol::{IpcRequest, OutgoingMessage};
use crate::application::state::{AppState, SharedState};

pub(crate) const PIPE_NAME: &str = r"\\.\pipe\isekai_daemon";

pub struct IpcServer {
    disk_manager: Arc<dyn DiskManager>,
    pipe_name: String,
    state: SharedState
}

impl IpcServer {
    pub fn new(disk_manager: Arc<dyn DiskManager>, pipe_name: impl Into<String>) -> Self {
        Self {
            disk_manager,
            pipe_name: pipe_name.into(),
            state: Arc::new(RwLock::new(AppState::default()))
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let mut server = ServerOptions::new()
            .first_pipe_instance(true)
            .create(&self.pipe_name)?;

        println!("IPC Server listening securely on Windows Named Pipe: {}", self.pipe_name);
        loop {
            server.connect().await?;
            println!("Client successfully connected via Named Pipe.");

            let client_pipe = server;
            server = ServerOptions::new().create(&self.pipe_name)?;

            let dm = self.disk_manager.clone();
            let state = self.state.clone();


            spawn(async move {
                let framed  = Framed::new(client_pipe, LengthDelimitedCodec::new());
                let (mut sink, mut stream) = framed.split();
                let (tx, mut rx) = channel::<OutgoingMessage>(100);

                let writer_task = spawn(async move {
                    while let Some(msg) = rx.recv().await {
                        if let Ok(json) = serde_json::to_string(&msg) {
                            if sink.send(Bytes::from(json)).await.is_err() {
                                break;
                            }
                        }
                    }
                });

                while let Some(result) = stream.next().await {
                    match result {
                        Ok(bytes_mut) => {
                            if let Ok(line) = String::from_utf8(bytes_mut.to_vec()) {
                                if let Ok(req) = serde_json::from_str::<IpcRequest>(&line) {
                                    let tx_clone = tx.clone();
                                    let dm_clone = dm.clone();
                                    let state_clone = state.clone();

                                    spawn(async move {
                                        let ctx = AppContext::IPC(tx_clone.clone(), state_clone.clone());

                                        APP_CONTEXT.scope(ctx, async move {
                                            process_request(req, dm_clone, tx_clone, state_clone).await;
                                        }).await;
                                    });
                                } else {
                                    eprintln!("Received malformed JSON frame payload: {}", line);
                                }
                            } else {
                                eprintln!("Received invalid non-UTF8 frame data.")
                            }
                        }
                        Err(e) => {
                            eprintln!("Named Pipe read boundary error: {}", e);
                            break;
                        }
                    }
                }

                writer_task.abort();
                println!("Client disconnected from Named Pipe.");
            });
        }
    }
}