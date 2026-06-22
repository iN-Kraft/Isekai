#[macro_export]
macro_rules! define_telemetry {
    (
        $(#[$router:meta])*
        pub enum $enum_name:ident {
            $(
                #[telemetry($lvl:ident, $msg:literal)]
                $variant:ident $({ $($field:ident : $ftype:ty),* $(,)? })?
            ),* $(,)?
        }
    ) => {
        $(#[$router])*
        #[derive(serde::Serialize, Debug, Clone)]
        #[serde(tag = "type")]
        pub enum $enum_name {
            $(
                $variant $({ $($field: $ftype),* })*
            ),*
        }

        impl $enum_name {
            #[allow(unused_variables)]
            pub fn message(&self) -> String {
                match self {
                    $(
                        $enum_name::$variant $({ $($field),* })* => format!($msg),
                    )*
                }
            }

            pub fn log_to_tracing(&self) {
                let msg = self.message();
                match self {
                    $(
                        $enum_name::$variant $({ $($field: _),* })* => {
                            $crate::telemetry_log_level!($lvl, &msg);
                        }
                    )*
                }
            }
        }
    };
}

#[macro_export]
macro_rules! telemetry_log_level {
    (error, $msg:expr) => { tracing::error!("{}", $msg) };
    (warn, $msg:expr) => { tracing::warn!("{}", $msg) };
    (info, $msg:expr) => { tracing::info!("{}", $msg) };
    (debug, $msg:expr) => { tracing::debug!("{}", $msg) };
    (trace, $msg:expr) => { tracing::trace!("{}", $msg) };

    (start, $msg:expr) => { tracing::info!("[START] {}", $msg) };
    (step, $msg:expr) => { tracing::info!(">>> {}", $msg) };
    (end, $msg:expr) => { tracing::info!("[END] {}", $msg) };

    (progress, $msg:expr) => { tracing::debug!("[PROGRESS] {}", $msg) };
}

#[macro_export]
macro_rules! telemetry {
    ($event:expr) => {
        let evt = $event;
        evt.log_to_tracing();

        let current_ctx = $crate::application::APP_CONTEXT.with(|ctx| ctx.clone());
        if let $crate::application::AppContext::IPC(tx) = current_ctx {
            let outgoing = $crate::ipc::protocol::OutgoingMessage::Event { payload: evt };
            let _ = tx.try_send(outgoing);
        }
    };
}