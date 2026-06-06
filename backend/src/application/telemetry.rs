#[macro_export]
macro_rules! telemetry {
    (start, $workflow:expr) => {
        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());
        let state = match &current_ctx {
            $crate::application::AppContext::CLI(s) => s,
            $crate::application::AppContext::IPC(_, s) => s
        };

        if let Ok(mut lock) = state.write() {
            lock.active_workflow = Some($workflow);
            lock.step_progress = Some(0);
            lock.step_details = Some("Initializing...".to_string());
        }

        match current_ctx {
            $crate::application::AppContext::CLI(_) => {
                tracing::info!("=== STARTING WORKFLOW: {:?} ===", $workflow);
            }
            $crate::application::AppContext::IPC(tx, _) => {
                let event = $crate::ipc::protocol::IpcEvent {
                    event_type: "start".to_string(),
                    message: "Initializing...".to_string(),
                    percent: Some(0),
                    workflow: Some($workflow.clone()), // PASS THE WORKFLOW HERE!
                };
                let _ = tx.try_send($crate::ipc::protocol::OutgoingMessage::Event(event));
            }
        }
    };

    (end) => {
        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());
        let state = match &current_ctx {
            $crate::application::AppContext::CLI(s) => s,
            $crate::application::AppContext::IPC(_, s) => s
        };

        let ended_workflow = if let Ok(mut lock) = state.write() {
            let w = lock.active_workflow.clone();
            lock.active_workflow = None;
            lock.step_progress = None;
            lock.step_details = None;
            w
        } else {
            None
        };

        match current_ctx {
            $crate::application::AppContext::CLI(_) => {
                tracing::info!("=== WORKFLOW ENDED ===");
            }
            $crate::application::AppContext::IPC(tx, _) => {
                let event = $crate::ipc::protocol::IpcEvent {
                    event_type: "end".to_string(),
                    message: "Workflow complete.".to_string(),
                    percent: None,
                    workflow: ended_workflow, // Optional: tell the GUI which workflow just finished
                };
                let _ = tx.try_send($crate::ipc::protocol::OutgoingMessage::Event(event));
            }
        }
    };

    (step, $msg:expr) => {
        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());

        let state = match &current_ctx {
            $crate::application::AppContext::CLI(s) => s,
            $crate::application::AppContext::IPC(_, s) => s
        };

        if let Ok(mut lock) = state.write() {
            lock.current_step = Some($msg.to_string());
            lock.step_details = None;
            lock.step_progress = None;
        }

        match current_ctx {
            $crate::application::AppContext::CLI(_) => {
                tracing::info!(">>> Step: {}", $msg);
            }
            $crate::application::AppContext::IPC(tx, _) => {
                let event = $crate::ipc::protocol::IpcEvent {
                    event_type: "step".to_string(),
                    message: $msg.to_string(),
                    percent: None,
                    workflow: None
                };
                let _ = tx.try_send($crate::ipc::protocol::OutgoingMessage::Event(event));
            }
        }
    };

    // Progress Events: ctx, progress, percent, "message", args
    (progress, $percent:expr, $details:expr $(, $args:expr)*) => {
        let formatted_msg = format!($details $(, $args)*);
        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());

        let state = match &current_ctx {
            $crate::application::AppContext::CLI(s) => s,
            $crate::application::AppContext::IPC(_, s) => s
        };

        if let Ok(mut lock) = state.write() {
            lock.step_progress = Some($percent as u8);
            lock.step_details = Some(formatted_msg.clone());
        }

        match current_ctx {
            $crate::application::AppContext::CLI(_) => {
                tracing::info!("[{}%] {}", $percent, formatted_msg);
            }
            $crate::application::AppContext::IPC(tx, _) => {
                let event = $crate::ipc::protocol::IpcEvent {
                    event_type: "progress".to_string(),
                    message: formatted_msg,
                    percent: Some($percent as u8),
                    workflow: None
                };

                let _ = tx.try_send($crate::ipc::protocol::OutgoingMessage::Event(event));
            }
        }
    };

    // Standard Events: ctx, info/warn/error, "message", args
    ($lvl:ident, $msg:expr $(, $args:expr)*) => {
        let formatted_msg = format!($msg $(, $args)*);
        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());

        let state = match &current_ctx {
            $crate::application::AppContext::CLI(s) => s,
            $crate::application::AppContext::IPC(_, s) => s
        };

        if let Ok(mut lock) = state.write() {
            lock.step_details = Some(formatted_msg.clone());
        }

        match current_ctx {
            $crate::application::AppContext::CLI(_) => {
                tracing::$lvl!("{}", formatted_msg);
            }
            $crate::application::AppContext::IPC(tx, _) => {
                let event = $crate::ipc::protocol::IpcEvent {
                    event_type: stringify!($lvl).to_string(),
                    message: formatted_msg,
                    percent: None,
                    workflow: None
                };

                let _ = tx.try_send($crate::ipc::protocol::OutgoingMessage::Event(event));
            }
        }
    }
}