#[cfg(windows)]
mod windows_app {
    use std::{
        sync::{Arc, Mutex, OnceLock},
        thread,
        time::Duration,
    };

    use chrono::{Local, Utc};
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::windows::named_pipe::ClientOptions,
        runtime::Builder,
    };
    use windows::{
        core::{w, PCWSTR, PWSTR},
        Win32::{
            Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, RECT, WPARAM},
            System::{LibraryLoader::GetModuleHandleW, Threading::CreateMutexW},
            UI::WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetDlgItem,
                GetMessageW, GetSystemMetrics, GetWindowTextLengthW, GetWindowTextW, LoadCursorW,
                PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer, SetWindowPos,
                SetWindowTextW, ShowWindow, TranslateMessage, BS_PUSHBUTTON, CREATESTRUCTW,
                CW_USEDEFAULT, ES_CENTER, ES_PASSWORD, GWLP_USERDATA, HMENU, HWND_TOPMOST,
                IDC_ARROW, MSG, MSGFLT_ALLOW, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN,
                SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SS_CENTER, SWP_NOACTIVATE, SWP_SHOWWINDOW,
                SW_HIDE, SW_SHOW, SYSTEM_PARAMETERS_INFO_UPDATE_FLAGS, WINDOW_EX_STYLE,
                WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE, WM_DESTROY, WM_SIZE, WM_TIMER,
                WNDCLASSW, WS_BORDER, WS_CHILD, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED,
                WS_POPUP, WS_VISIBLE,
            },
        },
    };
    use winpc_core::{DeviceMode, DeviceStatus, IpcRequest, IpcResponse};

    const PIPE_NAME: &str = r"\\.\pipe\WinParentalControlIpc";
    const WINDOW_CLASS: &str = "WinParentalControlAgentWindow";
    const TIMER_ID: usize = 1;
    const ID_PIN_EDIT: i32 = 2001;
    const ID_DURATION_EDIT: i32 = 2002;
    const ID_UNLOCK_BUTTON: i32 = 2003;
    const ID_TITLE_LABEL: i32 = 2004;
    const ID_MESSAGE_LABEL: i32 = 2005;
    const ID_HINT_LABEL: i32 = 2006;

    static UI_STATE: OnceLock<Arc<Mutex<UiState>>> = OnceLock::new();

    #[derive(Debug, Clone)]
    struct UiState {
        status: DeviceStatus,
        message: String,
        hwnd: HWND,
        title_hwnd: HWND,
        hint_hwnd: HWND,
        pin_hwnd: HWND,
        duration_hwnd: HWND,
        button_hwnd: HWND,
        message_hwnd: HWND,
        is_visible: bool,
    }

    impl Default for UiState {
        fn default() -> Self {
            Self {
                status: DeviceStatus {
                    mode: DeviceMode::Locked,
                    unlock_expires_at_utc: None,
                    remaining_minutes: 0,
                    agent_healthy: false,
                    protected_user_logged_in: true,
                    last_seen_at_utc: None,
                },
                message: "Waiting for service heartbeat...".to_string(),
                hwnd: HWND::default(),
                title_hwnd: HWND::default(),
                hint_hwnd: HWND::default(),
                pin_hwnd: HWND::default(),
                duration_hwnd: HWND::default(),
                button_hwnd: HWND::default(),
                message_hwnd: HWND::default(),
                is_visible: true,
            }
        }
    }

    pub fn run() -> Result<(), String> {
        let mutex = unsafe { CreateMutexW(None, true, w!("Global\\WinParentalControlAgent")) }
            .map_err(|error| error.to_string())?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return Ok(());
        }

        let state = Arc::new(Mutex::new(UiState::default()));
        let _ = UI_STATE.set(state.clone());
        spawn_polling_thread(state);
        run_message_loop()?;

        let _ = mutex;
        Ok(())
    }

    fn run_message_loop() -> Result<(), String> {
        let instance = unsafe { GetModuleHandleW(None) }.map_err(|error| error.to_string())?;
        let class_name = wide(WINDOW_CLASS);
        let cursor = unsafe { LoadCursorW(None, IDC_ARROW) }.map_err(|error| error.to_string())?;
        let class = WNDCLASSW {
            hCursor: cursor,
            hInstance: instance.into(),
            lpszClassName: PCWSTR(class_name.as_ptr()),
            lpfnWndProc: Some(window_proc),
            ..Default::default()
        };

        unsafe { RegisterClassW(&class) };
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        let hwnd = unsafe {
            CreateWindowExW(
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                PCWSTR(class_name.as_ptr()),
                w!("WinParentalControl"),
                WS_POPUP | WS_VISIBLE,
                x,
                y,
                width,
                height,
                None,
                None,
                instance,
                None,
            )
        }
        .map_err(|error| error.to_string())?;

        unsafe {
            ShowWindow(hwnd, SW_SHOW);
            SetForegroundWindow(hwnd);
            SetTimer(hwnd, TIMER_ID, 1000, None);
        }

        if let Some(state) = UI_STATE.get() {
            state.lock().unwrap().hwnd = hwnd;
        }

        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, HWND::default(), 0, 0) }.into() {
            unsafe {
                TranslateMessage(&message);
                DispatchMessageW(&message);
            }
        }

        Ok(())
    }

    fn spawn_polling_thread(state: Arc<Mutex<UiState>>) {
        thread::spawn(move || loop {
            let heartbeat = pipe_request(IpcRequest::Heartbeat);
            let state_response = pipe_request(IpcRequest::GetState);

            let mut ui = state.lock().unwrap();
            match (heartbeat, state_response) {
                (Ok(_), Ok(IpcResponse::State(status))) => {
                    ui.status = status;
                    ui.message = if ui.status.mode == DeviceMode::Locked {
                        "Locked. Parent PIN required.".to_string()
                    } else {
                        "Unlocked by parent.".to_string()
                    };
                }
                (Ok(IpcResponse::Ack(status)), _) => {
                    ui.status = status;
                }
                (_, Ok(IpcResponse::State(status))) => {
                    ui.status = status;
                    ui.message = "Service heartbeat failed; keeping screen locked.".to_string();
                }
                (Err(error), _) | (_, Err(error)) => {
                    ui.status.mode = DeviceMode::Locked;
                    ui.message = format!("Service unavailable: {error}");
                }
                (_, Ok(IpcResponse::Error { message })) => {
                    ui.status.mode = DeviceMode::Locked;
                    ui.message = message;
                }
                _ => {}
            }
            drop(ui);

            thread::sleep(Duration::from_secs(3));
        });
    }

    fn pipe_request(request: IpcRequest) -> Result<IpcResponse, String> {
        let runtime = Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|error| error.to_string())?;
        runtime.block_on(async move {
            let mut client = ClientOptions::new()
                .open(PIPE_NAME)
                .map_err(|error| error.to_string())?;
            let payload = format!(
                "{}\n",
                serde_json::to_string(&request).map_err(|error| error.to_string())?
            );
            client
                .write_all(payload.as_bytes())
                .await
                .map_err(|error| error.to_string())?;
            client.flush().await.map_err(|error| error.to_string())?;
            let mut reader = BufReader::new(client);
            let mut line = String::new();
            reader
                .read_line(&mut line)
                .await
                .map_err(|error| error.to_string())?;
            serde_json::from_str::<IpcResponse>(&line).map_err(|error| error.to_string())
        })
    }

    extern "system" fn window_proc(
        hwnd: HWND,
        message: u32,
        wparam: WPARAM,
        lparam: LPARAM,
    ) -> LRESULT {
        match message {
            WM_CREATE => {
                create_controls(hwnd);
                layout_controls(hwnd);
                refresh_ui();
                LRESULT(0)
            }
            WM_SIZE => {
                layout_controls(hwnd);
                LRESULT(0)
            }
            WM_TIMER => {
                refresh_ui();
                LRESULT(0)
            }
            WM_COMMAND => {
                let control_id = (wparam.0 & 0xffff) as i32;
                if control_id == ID_UNLOCK_BUTTON {
                    handle_unlock();
                }
                LRESULT(0)
            }
            WM_CLOSE => LRESULT(0),
            WM_DESTROY => {
                unsafe { PostQuitMessage(0) };
                LRESULT(0)
            }
            _ => unsafe { DefWindowProcW(hwnd, message, wparam, lparam) },
        }
    }

    fn create_controls(hwnd: HWND) {
        let title = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Screen locked"),
                WS_CHILD | WS_VISIBLE | SS_CENTER,
                0,
                0,
                100,
                40,
                hwnd,
                HMENU(ID_TITLE_LABEL as isize),
                None,
                None,
            )
        }
        .unwrap();
        let hint = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Parent PIN and duration are required to unlock."),
                WS_CHILD | WS_VISIBLE | SS_CENTER,
                0,
                0,
                100,
                40,
                hwnd,
                HMENU(ID_HINT_LABEL as isize),
                None,
                None,
            )
        }
        .unwrap();
        let pin = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!(""),
                WS_CHILD | WS_VISIBLE | WS_BORDER | ES_CENTER | ES_PASSWORD,
                0,
                0,
                100,
                40,
                hwnd,
                HMENU(ID_PIN_EDIT as isize),
                None,
                None,
            )
        }
        .unwrap();
        let duration = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!("30"),
                WS_CHILD | WS_VISIBLE | WS_BORDER | ES_CENTER,
                0,
                0,
                100,
                40,
                hwnd,
                HMENU(ID_DURATION_EDIT as isize),
                None,
                None,
            )
        }
        .unwrap();
        let button = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Unlock"),
                WS_CHILD | WS_VISIBLE | BS_PUSHBUTTON,
                0,
                0,
                100,
                40,
                hwnd,
                HMENU(ID_UNLOCK_BUTTON as isize),
                None,
                None,
            )
        }
        .unwrap();
        let message = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Waiting for service..."),
                WS_CHILD | WS_VISIBLE | SS_CENTER,
                0,
                0,
                100,
                60,
                hwnd,
                HMENU(ID_MESSAGE_LABEL as isize),
                None,
                None,
            )
        }
        .unwrap();

        if let Some(state) = UI_STATE.get() {
            let mut guard = state.lock().unwrap();
            guard.hwnd = hwnd;
            guard.title_hwnd = title;
            guard.hint_hwnd = hint;
            guard.pin_hwnd = pin;
            guard.duration_hwnd = duration;
            guard.button_hwnd = button;
            guard.message_hwnd = message;
        }
    }

    fn layout_controls(hwnd: HWND) {
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect) }.unwrap();
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let center_x = width / 2;
        let top = height / 2 - 140;
        let panel_width = 420;
        let left = center_x - panel_width / 2;

        move_control(ID_TITLE_LABEL, left, top, panel_width, 48);
        move_control(ID_HINT_LABEL, left, top + 56, panel_width, 32);
        move_control(ID_PIN_EDIT, left + 40, top + 112, 340, 36);
        move_control(ID_DURATION_EDIT, left + 40, top + 160, 340, 36);
        move_control(ID_UNLOCK_BUTTON, left + 40, top + 208, 340, 42);
        move_control(ID_MESSAGE_LABEL, left, top + 268, panel_width, 60);
    }

    fn move_control(id: i32, x: i32, y: i32, width: i32, height: i32) {
        if let Some(state) = UI_STATE.get() {
            let hwnd = state.lock().unwrap().hwnd;
            let child = unsafe { GetDlgItem(hwnd, id) };
            if child.0 != 0 {
                unsafe {
                    SetWindowPos(child, HWND::default(), x, y, width, height, SWP_NOACTIVATE)
                }
                .unwrap();
            }
        }
    }

    fn handle_unlock() {
        let Some(state) = UI_STATE.get() else {
            return;
        };
        let (pin_hwnd, duration_hwnd) = {
            let guard = state.lock().unwrap();
            (guard.pin_hwnd, guard.duration_hwnd)
        };

        let pin = read_control_text(pin_hwnd);
        let duration = read_control_text(duration_hwnd)
            .parse::<u16>()
            .unwrap_or(30);

        let result = pipe_request(IpcRequest::LocalUnlock {
            pin,
            duration_minutes: duration,
        });

        let mut guard = state.lock().unwrap();
        match result {
            Ok(IpcResponse::Ack(status)) => {
                guard.status = status;
                guard.message = "Unlocked locally.".to_string();
            }
            Ok(IpcResponse::Error { message }) => {
                guard.status.mode = DeviceMode::Locked;
                guard.message = message;
            }
            Err(error) => {
                guard.status.mode = DeviceMode::Locked;
                guard.message = error;
            }
            _ => {}
        }
    }

    fn read_control_text(hwnd: HWND) -> String {
        let len = unsafe { GetWindowTextLengthW(hwnd) };
        if len <= 0 {
            return String::new();
        }
        let mut buffer = vec![0u16; len as usize + 1];
        unsafe { GetWindowTextW(hwnd, PWSTR(buffer.as_mut_ptr()), buffer.len() as i32) };
        String::from_utf16_lossy(&buffer[..len as usize])
            .trim()
            .to_string()
    }

    fn refresh_ui() {
        let Some(state) = UI_STATE.get() else {
            return;
        };
        let mut guard = state.lock().unwrap();

        let title = match guard.status.mode {
            DeviceMode::Locked => "Screen locked".to_string(),
            DeviceMode::Unlocked => format!(
                "Unlocked until {}",
                guard
                    .status
                    .unlock_expires_at_utc
                    .map(|time| time
                        .with_timezone(&Local)
                        .format("%Y-%m-%d %H:%M")
                        .to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ),
        };

        let hint = format!(
            "Remaining: {} min | Agent: {}",
            guard.status.remaining_minutes,
            if guard.status.agent_healthy {
                "healthy"
            } else {
                "stale"
            }
        );
        let title_wide = wide(&title);
        let hint_wide = wide(&hint);
        let message_wide = wide(&guard.message);
        unsafe {
            SetWindowTextW(guard.title_hwnd, PCWSTR(title_wide.as_ptr())).unwrap();
            SetWindowTextW(guard.hint_hwnd, PCWSTR(hint_wide.as_ptr())).unwrap();
            SetWindowTextW(guard.message_hwnd, PCWSTR(message_wide.as_ptr())).unwrap();
        }

        if guard.status.mode == DeviceMode::Locked {
            if !guard.is_visible {
                unsafe { ShowWindow(guard.hwnd, SW_SHOW) };
                guard.is_visible = true;
            }
            unsafe {
                SetWindowPos(
                    guard.hwnd,
                    HWND_TOPMOST,
                    0,
                    0,
                    0,
                    0,
                    SWP_NOACTIVATE | SWP_SHOWWINDOW,
                )
                .unwrap();
                SetForegroundWindow(guard.hwnd);
            }
        } else if guard.is_visible {
            unsafe { ShowWindow(guard.hwnd, SW_HIDE) };
            guard.is_visible = false;
        }
    }

    fn wide(value: &str) -> Vec<u16> {
        value.encode_utf16().chain(std::iter::once(0)).collect()
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("winpc-agent only runs on Windows.");
}

#[cfg(windows)]
fn main() {
    if let Err(error) = windows_app::run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
