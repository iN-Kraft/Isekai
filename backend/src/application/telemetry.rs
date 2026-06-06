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
            lock.progress_percent = Some(0);
            lock.last_message = Some("Initializing...".to_string());
        }
    };

    (end) => {
        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());
        let state = match &current_ctx {
            $crate::application::AppContext::CLI(s) => s,
            $crate::application::AppContext::IPC(_, s) => s
        };

        if let Ok(mut lock) = state.write() {
            lock.active_workflow = None;
            lock.progress_percent = None;
            lock.last_message = None;
        }
    };

    // Progress Events: ctx, progress, percent, "message", args
    (progress, $percent:expr, $msg:expr $(, $args:expr)*) => {
        let formatted_msg = format!($msg $(, $args)*);
        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());

        let state = match &current_ctx {
            $crate::application::AppContext::CLI(s) => s,
            $crate::application::AppContext::IPC(_, s) => s
        };

        if let Ok(mut lock) = state.write() {
            lock.progress_percent = Some($percent as u8);
            lock.last_message = Some(formatted_msg.clone());
        }

        match current_ctx {
            $crate::application::AppContext::CLI(_) => {
                tracing::info!("[{}%] {}", $percent, formatted_msg);
            }
            $crate::application::AppContext::IPC(tx, _) => {
                let event = $crate::ipc::protocol::IpcEvent {
                    event_type: "progress".to_string(),
                    message: formatted_msg,
                    percent: Some($percent as u8)
                };

                let _ = tx.send($crate::ipc::protocol::OutgoingMessage::Event(event)).await;
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
            lock.last_message = Some(formatted_msg.clone());
        }

        match current_ctx {
            $crate::application::AppContext::CLI(_) => {
                tracing::$lvl!("{}", formatted_msg);
            }
            $crate::application::AppContext::IPC(tx, _) => {
                let event = $crate::ipc::protocol::IpcEvent {
                    event_type: stringify!($lvl).to_string(),
                    message: formatted_msg,
                    percent: None
                };

                let _ = tx.send($crate::ipc::protocol::OutgoingMessage::Event(event)).await;
            }
        }
    }
}