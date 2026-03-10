#[cfg(windows)]
mod imp {
    use std::{ffi::OsStr, os::windows::ffi::OsStrExt, path::PathBuf, sync::mpsc};

    use chrono::Utc;
    use tokio::runtime::Builder;
    use tracing::{error, info};
    use windows::{
        core::{PCWSTR, PWSTR},
        Win32::{
            Foundation::{CloseHandle, HANDLE, HLOCAL},
            Security::{
                Authorization::ConvertSidToStringSidW, CreateProcessAsUserW, GetTokenInformation,
                TokenUser, TOKEN_INFORMATION_CLASS, TOKEN_USER,
            },
            System::{
                Memory::LocalFree,
                RemoteDesktop::{WTSGetActiveConsoleSessionId, WTSQueryUserToken},
                Threading::{PROCESS_CREATION_FLAGS, PROCESS_INFORMATION, STARTUPINFOW},
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

    use crate::state::SharedState;

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
        let is_logged_in = match protected_sid.as_deref() {
            Some(sid) => active_console_matches_sid(sid)?,
            None => false,
        };

        state.set_protected_user_logged_in(is_logged_in).await;
        let status = state.mark_agent_unhealthy_if_needed().await;
        if is_logged_in && !status.agent_healthy && state.should_retry_agent_spawn(Utc::now()).await
        {
            if let Some(sid) = protected_sid.as_deref() {
                spawn_agent_for_active_console_user(sid)?;
            }
        }

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

    fn spawn_agent_for_active_console_user(expected_sid: &str) -> std::io::Result<()> {
        if !active_console_matches_sid(expected_sid)? {
            return Ok(());
        }

        let session_id = unsafe { WTSGetActiveConsoleSessionId() };
        if session_id == u32::MAX {
            return Ok(());
        }

        let mut token = HANDLE::default();
        unsafe { WTSQueryUserToken(session_id, &mut token) }.ok()?;

        let agent_path = current_agent_path()?;
        let application = wide(agent_path.as_os_str());
        let desktop = wide(OsStr::new("winsta0\\default"));
        let mut startup_info = STARTUPINFOW::default();
        startup_info.cb = std::mem::size_of::<STARTUPINFOW>() as u32;
        startup_info.lpDesktop = PWSTR(desktop.as_ptr() as *mut _);
        let mut process_info = PROCESS_INFORMATION::default();

        let result = unsafe {
            CreateProcessAsUserW(
                token,
                PCWSTR(application.as_ptr()),
                PWSTR::null(),
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
            if process_info.hProcess != HANDLE::default() {
                let _ = CloseHandle(process_info.hProcess);
            }
            if process_info.hThread != HANDLE::default() {
                let _ = CloseHandle(process_info.hThread);
            }
        }

        result.ok()?;
        info!("spawned winpc-agent for active console session");
        Ok(())
    }

    fn current_agent_path() -> std::io::Result<PathBuf> {
        let current = std::env::current_exe()?;
        Ok(current.with_file_name("winpc-agent.exe"))
    }

    fn active_console_user_sid() -> std::io::Result<Option<String>> {
        let session_id = unsafe { WTSGetActiveConsoleSessionId() };
        if session_id == u32::MAX {
            return Ok(None);
        }

        let mut token = HANDLE::default();
        let result = unsafe { WTSQueryUserToken(session_id, &mut token) };
        if let Err(error) = result.ok() {
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

    fn token_user_sid(token: HANDLE) -> std::io::Result<String> {
        let mut size = 0u32;
        unsafe {
            let _ = GetTokenInformation(
                token,
                TOKEN_INFORMATION_CLASS(TokenUser.0),
                None,
                0,
                &mut size,
            );
        }

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
        .ok()
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;

        let token_user = unsafe { &*(buffer.as_ptr() as *const TOKEN_USER) };
        let mut sid_ptr = PWSTR::null();
        unsafe { ConvertSidToStringSidW(token_user.User.Sid, &mut sid_ptr) }
            .ok()
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;
        let sid = sid_ptr
            .to_string()
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::Other, error.to_string()))?;
        unsafe {
            let _ = LocalFree(HLOCAL(sid_ptr.0 as isize));
        }
        Ok(sid)
    }

    fn wide(value: &OsStr) -> Vec<u16> {
        value.encode_wide().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
mod imp {
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
}

pub use imp::{run, supervisor_tick};
