#[cfg(not(windows))]
use std::future::pending;

#[cfg(windows)]
mod imp {
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::windows::named_pipe::ServerOptions,
    };
    use tracing::error;
    use winpc_core::{IpcRequest, IpcResponse};

    use crate::state::SharedState;

    const PIPE_NAME: &str = r"\\.\pipe\WinParentalControlIpc";

    pub async fn run_ipc_server(
        state: SharedState,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        loop {
            let server = ServerOptions::new().create(PIPE_NAME)?;
            server.connect().await?;
            let state = state.clone();
            tokio::spawn(async move {
                if let Err(error) = handle_client(server, state).await {
                    error!("ipc client failed: {error}");
                }
            });
        }
    }

    async fn handle_client(
        pipe: tokio::net::windows::named_pipe::NamedPipeServer,
        state: SharedState,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let (read_half, mut write_half) = tokio::io::split(pipe);
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        reader.read_line(&mut line).await?;
        if line.trim().is_empty() {
            return Ok(());
        }

        let request: IpcRequest = serde_json::from_str(&line)?;
        let response = match request {
            IpcRequest::GetState => IpcResponse::State(state.device_status().await),
            IpcRequest::Heartbeat => IpcResponse::Ack(state.record_heartbeat().await?),
            IpcRequest::LocalUnlock {
                pin,
                duration_minutes,
            } => match state.local_unlock(&pin, duration_minutes).await {
                Ok(status) => IpcResponse::Ack(status),
                Err(error) => IpcResponse::Error {
                    message: error.to_string(),
                },
            },
        };

        write_half
            .write_all(format!("{}\n", serde_json::to_string(&response)?).as_bytes())
            .await?;
        write_half.flush().await?;
        Ok(())
    }
}

#[cfg(not(windows))]
mod imp {
    use super::pending;

    use crate::state::SharedState;

    pub async fn run_ipc_server(
        _state: SharedState,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        pending::<()>().await;
        Ok(())
    }
}

pub use imp::run_ipc_server;
