use std::sync::{Arc, RwLock};
use crate::domain::traits::DiskManager;
use std::error::Error;
use std::ffi::c_void;
use std::ptr::null_mut;
use futures::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio::net::windows::named_pipe::{ServerOptions, NamedPipeServer};
use tokio::spawn;
use tokio::sync::mpsc::channel;
use tokio_util::bytes::Bytes;
use tokio_util::codec::{Framed, LengthDelimitedCodec, LinesCodec};
use windows_sys::Win32::Foundation::{CloseHandle, LocalFree, INVALID_HANDLE_VALUE};
use windows_sys::Win32::Security::Authorization::{ConvertStringSecurityDescriptorToSecurityDescriptorW, SDDL_REVISION_1};
use windows_sys::Win32::Security::SECURITY_ATTRIBUTES;
use windows_sys::Win32::System::Threading::{OpenEventW, SetEvent, EVENT_MODIFY_STATE};
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

    fn create_pipe_with_security(pipe_name: &str, first_instance: bool) -> std::io::Result<NamedPipeServer> {
        unsafe {
            // SDDL: Allow (A) Generic All (GA) to Everyone (WD)
            let sddl: Vec<u16> = "D:(A;;GA;;;WD)\0".encode_utf16().collect();
            let mut sd_ptr = null_mut();
            let success = ConvertStringSecurityDescriptorToSecurityDescriptorW(
                sddl.as_ptr(),
                SDDL_REVISION_1,
                &mut sd_ptr,
                null_mut()
            );

            if success == 0 {
                return Err(std::io::Error::last_os_error());
            }

            let mut sa = SECURITY_ATTRIBUTES {
                nLength: size_of::<SECURITY_ATTRIBUTES>() as u32,
                lpSecurityDescriptor: sd_ptr,
                bInheritHandle: 0
            };

            let server_result = ServerOptions::new()
                .first_pipe_instance(first_instance)
                .create_with_security_attributes_raw(pipe_name, &mut sa as *mut _ as *mut c_void);

            LocalFree(sd_ptr as _);

            server_result
        }
    }

    pub async fn start(&self) -> Result<(), Box<dyn Error>> {
        let mut server = Self::create_pipe_with_security(&self.pipe_name, true)?;

        println!("IPC Server listening securely on Windows Named Pipe: {}", self.pipe_name);
        self.signal_frontend_ready();

        loop {
            server.connect().await?;
            println!("Client successfully connected via Named Pipe.");

            let client_pipe = server;
            server = Self::create_pipe_with_security(&self.pipe_name, false)?;

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

    fn signal_frontend_ready(&self) {
        let event_name: Vec<u16> = "Local\\IsekaiDaemonReady\0".encode_utf16().collect();
        unsafe {
            let h_event = OpenEventW(EVENT_MODIFY_STATE, 0, event_name.as_ptr());
            if h_event != INVALID_HANDLE_VALUE {
                SetEvent(h_event);
                CloseHandle(h_event);
            } else {
                eprintln!("Warning: Could not find frontend sync event.")
            }
        }
    }
}