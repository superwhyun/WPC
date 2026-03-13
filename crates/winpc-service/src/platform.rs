#[cfg(windows)]
mod imp {
    use std::{
        ffi::OsStr,
        os::windows::ffi::OsStrExt,
        path::{Path, PathBuf},
        sync::mpsc,
    };

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use chrono::Utc;
    use tokio::runtime::Builder;
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::windows::named_pipe::ClientOptions,
    };
    use tracing::{error, info, warn};
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::{
            Foundation::{CloseHandle, LocalFree, HANDLE, HLOCAL},
            Security::{
                Authorization::ConvertSidToStringSidW, GetTokenInformation, TokenUser,
                TOKEN_INFORMATION_CLASS, TOKEN_QUERY, TOKEN_USER,
            },
            System::{
                RemoteDesktop::{
                    ProcessIdToSessionId, WTSGetActiveConsoleSessionId, WTSQueryUserToken,
                },
                Threading::{
                    CreateProcessAsUserW, GetCurrentProcess, GetCurrentProcessId,
                    GetExitCodeProcess, OpenProcessToken, WaitForSingleObject, INFINITE,
                    PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW,
                },
            },
        },
    };
    use windows_service::{
        define_windows_service,
        service::{
            ServiceControl, ServiceControlAccept, ServiceExitCode, ServiceState, ServiceStatus,
            ServiceType,
        },
        service_control_handler::{self, ServiceControlHandlerResult},
        service_dispatcher,
    };
    use winpc_core::{
        AgentCommandRequest, AgentCommandResponse, Error, Result, UnlockExpiryAction,
        AGENT_COMMAND_PIPE_NAME,
    };

    use crate::state::{PendingExpiryAction, SharedState};

    const SERVICE_NAME: &str = "WinParentalControlService";

    define_windows_service!(ffi_service_main, service_main);

    pub fn run() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        if std::env::args().any(|arg| arg == "--console") {
            return block_on_console();
        }

        match service_dispatcher::start(SERVICE_NAME, ffi_service_main) {
            Ok(()) => Ok(()),
            Err(error) => {
                eprintln!("service dispatcher unavailable ({error}), falling back to console mode");
                block_on_console()
            }
        }
    }

    pub async fn supervisor_tick(
        state: &SharedState,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let config = state.current_config().await;
        let protected_sid = config.protected_user_sid.clone();
        let active_console_sid = if is_console_mode() {
            current_process_user_sid().map(Some)?
        } else {
            active_console_user_sid()?
        };
        let is_logged_in = match protected_sid.as_deref() {
            Some(sid) => active_console_sid.as_deref() == Some(sid),
            None => active_console_sid.is_some(),
        };

        state.set_protected_user_logged_in(is_logged_in).await;
        if let Some(pending) = state.take_pending_expiry_action().await? {
            perform_pending_expiry_action(pending)?;
        }
        let status = state.mark_agent_unhealthy_if_needed().await;
        if is_logged_in && !status.agent_healthy && state.should_retry_agent_spawn(Utc::now()).await
        {
            warn!(
                active_console_sid = ?active_console_sid,
                protected_sid = ?protected_sid,
                "agent is unhealthy; attempting to launch winpc-agent"
            );
            spawn_agent_for_active_console_user(protected_sid.as_deref())?;
        }

        Ok(())
    }

    fn perform_pending_expiry_action(pending: PendingExpiryAction) -> std::io::Result<()> {
        if pending.warn_only {
            warn!(
                action = ?pending.action,
                "unlock timer expired in warn-only mode; skipping follow-up action"
            );
            return Ok(());
        }

        match pending.action {
            UnlockExpiryAction::AppLock => {
                warn!("unlock timer expired; app lock is active again");
                Ok(())
            }
            UnlockExpiryAction::WindowsLock => {
                warn!("unlock timer expired; requesting Windows lock");
                lock_active_console(pending.protected_user_sid.as_deref())
            }
            UnlockExpiryAction::Shutdown => {
                warn!("unlock timer expired; requesting system shutdown");
                shutdown_machine()
            }
        }
    }

    pub fn lock_active_console(expected_sid: Option<&str>) -> std::io::Result<()> {
        let rundll32 = system_path("rundll32.exe");
        
        // Console mode: run directly in current process (no SYSTEM permission needed)
        if is_console_mode() {
            if let Some(expected_sid) = expected_sid {
                if !active_console_matches_sid(expected_sid).unwrap_or(false) {
                    return Err(std::io::Error::other(
                        "active console session does not match the protected user",
                    ));
                }
            }
            std::process::Command::new(&rundll32)
                .arg("user32.dll,LockWorkStation")
                .spawn()?;
            return Ok(());
        }
        
        // Service mode: try to run in active console session, fallback if needed
        let command_line = format!("\"{}\" user32.dll,LockWorkStation", rundll32.display());
        let can_fallback = can_run_in_current_process(expected_sid).unwrap_or(false);
        match run_in_active_console_session(expected_sid, &rundll32, &command_line, false) {
            Ok(()) => {}
            Err(error) if can_fallback => {
                std::process::Command::new(&rundll32)
                    .arg("user32.dll,LockWorkStation")
                    .spawn()?;
                info!("fell back to current-process workstation lock: {error}");
            }
            Err(error) => return Err(error),
        }
        Ok(())
    }

    pub fn shutdown_machine() -> std::io::Result<()> {
        std::process::Command::new(system_path("shutdown.exe"))
            .args(["/s", "/t", "0", "/f"])
            .spawn()?;
        Ok(())
    }

    fn block_on_console() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let runtime = Builder::new_multi_thread().enable_all().build()?;
        runtime.block_on(crate::run_console())
    }

    fn service_main(_arguments: Vec<std::ffi::OsString>) {
        if let Err(error) = run_service() {
            error!("windows service failed: {error}");
        }
    }

    fn run_service() -> windows_service::Result<()> {
        let (stop_tx, stop_rx) = mpsc::channel::<()>();
        let status_handle =
            service_control_handler::register(SERVICE_NAME, move |event| match event {
                ServiceControl::Stop | ServiceControl::Shutdown => {
                    let _ = stop_tx.send(());
                    ServiceControlHandlerResult::NoError
                }
                ServiceControl::Interrogate => ServiceControlHandlerResult::NoError,
                _ => ServiceControlHandlerResult::NotImplemented,
            })?;

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Running,
            controls_accepted: ServiceControlAccept::STOP | ServiceControlAccept::SHUTDOWN,
            exit_code: ServiceExitCode::Win32(0),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })?;

        let runtime = Builder::new_multi_thread()
            .enable_all()
            .build()
            .expect("failed to build tokio runtime");

        let config_path = std::env::var_os("WINPC_CONFIG_PATH")
            .map_or_else(winpc_core::config::default_config_path, Into::into);
        let result = runtime.block_on(crate::run_with_shutdown_signal(config_path, async move {
            let _ = tokio::task::spawn_blocking(move || stop_rx.recv()).await;
        }));

        let exit_code = if result.is_ok() { 0 } else { 1 };
        if let Err(error) = &result {
            error!("service runtime exited with error: {error}");
        }

        status_handle.set_service_status(ServiceStatus {
            service_type: ServiceType::OWN_PROCESS,
            current_state: ServiceState::Stopped,
            controls_accepted: ServiceControlAccept::empty(),
            exit_code: ServiceExitCode::Win32(exit_code),
            checkpoint: 0,
            wait_hint: std::time::Duration::default(),
            process_id: None,
        })?;

        Ok(())
    }

    fn active_console_matches_sid(expected_sid: &str) -> std::io::Result<bool> {
        Ok(active_console_user_sid()?.as_deref() == Some(expected_sid))
    }

    fn can_run_in_current_process(expected_sid: Option<&str>) -> std::io::Result<bool> {
        if !current_process_is_active_console_session()? {
            return Ok(false);
        }

        match expected_sid {
            Some(expected_sid) => Ok(current_process_user_sid()? == expected_sid),
            None => Ok(true),
        }
    }

    fn spawn_agent_for_active_console_user(expected_sid: Option<&str>) -> std::io::Result<()> {
        let agent_path = current_agent_path()?;
        let command_line = format!("\"{}\"", agent_path.display());
        if is_console_mode() {
            if let Some(expected_sid) = expected_sid {
                let current_sid = current_process_user_sid()?;
                if current_sid != expected_sid {
                    return Err(std::io::Error::other(
                        "current console user does not match the protected user",
                    ));
                }
            }
            warn!(
                agent_path = %agent_path.display(),
                expected_sid = ?expected_sid,
                "launching winpc-agent directly in the current console session"
            );
            std::process::Command::new(&agent_path).spawn()?;
            warn!("spawned winpc-agent in current console session");
            return Ok(());
        }

        let can_fallback = can_run_in_current_process(expected_sid).unwrap_or(false);
        warn!(
            agent_path = %agent_path.display(),
            expected_sid = ?expected_sid,
            can_fallback,
            "launching winpc-agent"
        );
        match run_in_active_console_session(expected_sid, &agent_path, &command_line, false) {
            Ok(()) => {}
            Err(error) if can_fallback => {
                std::process::Command::new(&agent_path).spawn()?;
                info!("fell back to current-process agent launch: {error}");
            }
            Err(error) => return Err(error),
        }
        warn!("spawned winpc-agent for active console session");
        Ok(())
    }

    fn is_console_mode() -> bool {
        std::env::args().any(|arg| arg == "--console")
    }

    fn current_agent_path() -> std::io::Result<PathBuf> {
        let current = std::env::current_exe()?;
        Ok(current.with_file_name("winpc-agent.exe"))
    }

    pub async fn capture_snapshot() -> Result<Vec<u8>> {
        let mut client = ClientOptions::new()
            .open(AGENT_COMMAND_PIPE_NAME)
            .map_err(|error| Error::SnapshotUnavailable(error.to_string()))?;
        let payload = format!(
            "{}\n",
            serde_json::to_string(&AgentCommandRequest::CaptureSnapshot)
                .map_err(|error| Error::SnapshotUnavailable(error.to_string()))?
        );
        client
            .write_all(payload.as_bytes())
            .await
            .map_err(|error| Error::SnapshotUnavailable(error.to_string()))?;
        client
            .flush()
            .await
            .map_err(|error| Error::SnapshotUnavailable(error.to_string()))?;

        let mut reader = BufReader::new(client);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|error| Error::SnapshotUnavailable(error.to_string()))?;

        let response: AgentCommandResponse = serde_json::from_str(&line)
            .map_err(|error| Error::SnapshotUnavailable(error.to_string()))?;

        match response {
            AgentCommandResponse::Snapshot { png_base64 } => STANDARD
                .decode(png_base64)
                .map_err(|error| Error::SnapshotUnavailable(error.to_string())),
            AgentCommandResponse::Error { message } => Err(Error::SnapshotUnavailable(message)),
        }
    }

    fn active_console_user_sid() -> std::io::Result<Option<String>> {
        let session_id = unsafe { WTSGetActiveConsoleSessionId() };
        if session_id == u32::MAX {
            return Ok(None);
        }

        let mut token = HANDLE::default();
        let result = unsafe { WTSQueryUserToken(session_id, &mut token) };
        if let Err(error) = result {
            if current_process_is_active_console_session()? {
                info!(
                    "falling back to current-process SID for active console detection: {}",
                    error
                );
                return current_process_user_sid().map(Some);
            }

            return Err(std::io::Error::new(
                std::io::ErrorKind::Other,
                error.to_string(),
            ));
        }

        let sid = token_user_sid(token);
        unsafe {
            let _ = CloseHandle(token);
        }
        sid.map(Some)
    }

    fn current_process_is_active_console_session() -> std::io::Result<bool> {
        let active_session_id = unsafe { WTSGetActiveConsoleSessionId() };
        if active_session_id == u32::MAX {
            return Ok(false);
        }

        let mut session_id = 0u32;
        unsafe { ProcessIdToSessionId(GetCurrentProcessId(), &mut session_id) }
            .map_err(|error: windows::core::Error| std::io::Error::other(error.to_string()))?;
        Ok(session_id == active_session_id)
    }

    fn current_process_user_sid() -> std::io::Result<String> {
        let mut token = HANDLE::default();
        unsafe { OpenProcessToken(GetCurrentProcess(), TOKEN_QUERY, &mut token) }
            .map_err(|error: windows::core::Error| std::io::Error::other(error.to_string()))?;

        let sid = token_user_sid(token);
        unsafe {
            let _ = CloseHandle(token);
        }
        sid
    }

    fn token_user_sid(token: HANDLE) -> std::io::Result<String> {
        let mut size = 0u32;
        let _ = unsafe {
            GetTokenInformation(
                token,
                TOKEN_INFORMATION_CLASS(TokenUser.0),
                None,
                0,
                &mut size,
            )
        };

        let mut buffer = vec![0u8; size as usize];
        unsafe {
            GetTokenInformation(
                token,
                TOKEN_INFORMATION_CLASS(TokenUser.0),
                Some(buffer.as_mut_ptr() as *mut _),
                size,
                &mut size,
            )
        }
        .map_err(|error| std::io::Error::other(error.to_string()))?;

        let token_user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
        let mut sid_ptr = PWSTR::null();
        unsafe { ConvertSidToStringSidW(token_user.User.Sid, &mut sid_ptr) }
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        let sid = unsafe { sid_ptr.to_string() }
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        unsafe {
            let _ = LocalFree(Some(HLOCAL(sid_ptr.0.cast())));
        }
        Ok(sid)
    }

    fn run_in_active_console_session(
        expected_sid: Option<&str>,
        application: &Path,
        command_line: &str,
        wait: bool,
    ) -> std::io::Result<()> {
        if let Some(expected_sid) = expected_sid {
            if !active_console_matches_sid(expected_sid)? {
                return Err(std::io::Error::other(
                    "active console session does not match the protected user",
                ));
            }
        }

        let session_id = unsafe { WTSGetActiveConsoleSessionId() };
        if session_id == u32::MAX {
            return Err(std::io::Error::other("no active console session"));
        }

        let mut token = HANDLE::default();
        unsafe { WTSQueryUserToken(session_id, &mut token) }
            .map_err(|error| std::io::Error::other(error.to_string()))?;

        let application = wide(application.as_os_str());
        let mut command_line = wide(OsStr::new(command_line));
        let desktop = wide(OsStr::new("winsta0\\default"));
        let mut startup_info = STARTUPINFOW::default();
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        startup_info.lpDesktop = PWSTR(desktop.as_ptr() as *mut _);
        let mut process_info = PROCESS_INFORMATION::default();

        let result = unsafe {
            CreateProcessAsUserW(
                Some(token),
                PCWSTR(application.as_ptr()),
                Some(PWSTR(command_line.as_mut_ptr())),
                None,
                None,
                false,
                PROCESS_CREATION_FLAGS(0),
                None,
                PCWSTR::null(),
                &startup_info,
                &mut process_info,
            )
        };

        unsafe {
            let _ = CloseHandle(token);
        }

        result.map_err(|error| std::io::Error::other(error.to_string()))?;

        if wait {
            wait_for_process(process_info.hProcess)?;
        }

        unsafe {
            if process_info.hProcess != HANDLE::default() {
                let _ = CloseHandle(process_info.hProcess);
            }
            if process_info.hThread != HANDLE::default() {
                let _ = CloseHandle(process_info.hThread);
            }
        }

        Ok(())
    }

    fn wait_for_process(process: HANDLE) -> std::io::Result<()> {
        unsafe {
            let _ = WaitForSingleObject(process, INFINITE);
            let mut exit_code = 0u32;
            GetExitCodeProcess(process, &mut exit_code)
                .map_err(|error| std::io::Error::other(error.to_string()))?;
            if exit_code != 0 {
                return Err(std::io::Error::other(format!(
                    "helper exited with status {exit_code}"
                )));
            }
        }
        Ok(())
    }

    fn system_path(relative: &str) -> PathBuf {
        let root = std::env::var_os("SystemRoot").unwrap_or_else(|| "C:\\Windows".into());
        PathBuf::from(root).join("System32").join(relative)
    }

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
mod imp {
    use winpc_core::{Error, Result};

    use crate::state::SharedState;

    pub fn run() -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        let runtime = tokio::runtime::Builder::new_multi_thread()
            .enable_all()
            .build()?;
        runtime.block_on(crate::run_console())
    }

    pub async fn supervisor_tick(
        state: &SharedState,
    ) -> std::result::Result<(), Box<dyn std::error::Error + Send + Sync>> {
        state.set_protected_user_logged_in(false).await;
        let _ = state.mark_agent_unhealthy_if_needed().await;
        Ok(())
    }

    pub fn lock_active_console(_expected_sid: Option<&str>) -> std::io::Result<()> {
        Err(std::io::Error::other(
            "windows lock is only supported on Windows",
        ))
    }

    pub fn shutdown_machine() -> std::io::Result<()> {
        Err(std::io::Error::other(
            "shutdown is only supported on Windows",
        ))
    }

    pub async fn capture_snapshot() -> Result<Vec<u8>> {
        Err(Error::SnapshotUnavailable(
            "snapshot is only available on Windows".to_string(),
        ))
    }
}

pub use imp::{capture_snapshot, lock_active_console, run, shutdown_machine, supervisor_tick};
