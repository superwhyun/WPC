#[cfg(windows)]
mod command_pipe;

#[cfg(windows)]
mod windows_app {
    use std::{
        ffi::c_void,
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
        core::{w, PCWSTR},
        Win32::{
            Foundation::{GetLastError, ERROR_ALREADY_EXISTS, HWND, LPARAM, LRESULT, RECT, WPARAM},
            Graphics::Gdi::{
                CreateFontW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, DEFAULT_CHARSET,
                DEFAULT_PITCH, FF_MODERN, FF_SWISS, FONT_CLIP_PRECISION,
                FONT_OUTPUT_PRECISION, FW_BOLD, FW_NORMAL, HFONT, OUT_DEFAULT_PRECIS,
            },
            System::{
                LibraryLoader::GetModuleHandleW,
                SystemServices::{STATIC_STYLES, SS_CENTER},
                Threading::CreateMutexW,
            },
            UI::WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DispatchMessageW, GetClientRect, GetDlgItem,
                GetMessageW, GetSystemMetrics, GetWindowTextLengthW, GetWindowTextW, LoadCursorW,
                PostQuitMessage, RegisterClassW, SetForegroundWindow, SetTimer, SetWindowPos,
                SetWindowTextW, ShowWindow, TranslateMessage, BS_PUSHBUTTON, ES_CENTER, ES_PASSWORD,
                HMENU, HWND_TOPMOST, IDC_ARROW, MSG, SendMessageW, SM_CXVIRTUALSCREEN,
                SM_CYVIRTUALSCREEN,
                SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE,
                SW_SHOW, WINDOW_EX_STYLE, WINDOW_STYLE, WM_CLOSE, WM_COMMAND, WM_CREATE,
                WM_DESTROY, WM_SETFONT, WM_SIZE, WM_TIMER, WNDCLASSW, WS_BORDER, WS_CHILD,
                WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_OVERLAPPED, WS_VISIBLE,
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
    const ID_TIMER_LABEL: i32 = 2007;
    const WARN_ONLY_WIDTH: i32 = 560;
    const WARN_ONLY_HEIGHT: i32 = 380;

    static UI_STATE: OnceLock<Arc<Mutex<UiState>>> = OnceLock::new();

    #[derive(Debug, Clone)]
    struct UiState {
        status: DeviceStatus,
        message: String,
        hwnd: isize,
        title_hwnd: isize,
        timer_hwnd: isize,
        hint_hwnd: isize,
        pin_hwnd: isize,
        duration_hwnd: isize,
        button_hwnd: isize,
        message_hwnd: isize,
        title_font: isize,
        timer_font: isize,
        body_font: isize,
        is_visible: bool,
    }

    impl Default for UiState {
        fn default() -> Self {
            Self {
                status: DeviceStatus {
                    mode: DeviceMode::Locked,
                    warn_only: false,
                    unlock_expires_at_utc: None,
                    remaining_minutes: 0,
                    agent_healthy: false,
                    protected_user_logged_in: true,
                    last_seen_at_utc: None,
                },
                message: "Waiting for service heartbeat...".to_string(),
                hwnd: 0,
                title_hwnd: 0,
                timer_hwnd: 0,
                hint_hwnd: 0,
                pin_hwnd: 0,
                duration_hwnd: 0,
                button_hwnd: 0,
                message_hwnd: 0,
                title_font: 0,
                timer_font: 0,
                body_font: 0,
                is_visible: true,
            }
        }
    }

    fn hwnd_to_raw(hwnd: HWND) -> isize {
        hwnd.0 as isize
    }

    fn hwnd_from_raw(raw: isize) -> HWND {
        HWND(raw as *mut c_void)
    }

    fn hmenu_from_id(id: i32) -> HMENU {
        HMENU(id as usize as *mut c_void)
    }

    fn hfont_to_raw(font: HFONT) -> isize {
        font.0 as isize
    }

    fn child_style(extra: u32) -> WINDOW_STYLE {
        WINDOW_STYLE(WS_CHILD.0 | WS_VISIBLE.0 | extra)
    }

    fn static_center_style() -> WINDOW_STYLE {
        child_style(STATIC_STYLES(SS_CENTER.0).0)
    }

    fn countdown_text(status: &DeviceStatus) -> String {
        let Some(expires_at) = status.unlock_expires_at_utc else {
            return "00:00".to_string();
        };
        let remaining = (expires_at - Utc::now()).num_seconds().max(0);
        let hours = remaining / 3600;
        let minutes = (remaining % 3600) / 60;
        let seconds = remaining % 60;
        if hours > 0 {
            format!("{hours:02}:{minutes:02}:{seconds:02}")
        } else {
            format!("{minutes:02}:{seconds:02}")
        }
    }

    fn create_font(height: i32, weight: i32, family: u32, face: PCWSTR) -> HFONT {
        unsafe {
            CreateFontW(
                height,
                0,
                0,
                0,
                weight,
                0,
                0,
                0,
                DEFAULT_CHARSET,
                FONT_OUTPUT_PRECISION(OUT_DEFAULT_PRECIS.0),
                FONT_CLIP_PRECISION(CLIP_DEFAULT_PRECIS.0),
                CLEARTYPE_QUALITY,
                DEFAULT_PITCH.0 as u32 | family,
                face,
            )
        }
    }

    fn apply_font(hwnd: HWND, font: HFONT) {
        unsafe {
            let _ = SendMessageW(
                hwnd,
                WM_SETFONT,
                Some(WPARAM(font.0 as usize)),
                Some(LPARAM(1)),
            );
        }
    }

    pub fn run() -> Result<(), String> {
        let mutex = unsafe { CreateMutexW(None, true, w!("Global\\WinParentalControlAgent")) }
            .map_err(|error| error.to_string())?;
        if unsafe { GetLastError() } == ERROR_ALREADY_EXISTS {
            return Ok(());
        }

        let mut initial_state = UiState::default();
        if let Ok(IpcResponse::State(status) | IpcResponse::Ack(status)) =
            pipe_request(IpcRequest::GetState)
        {
            initial_state.status = status;
            initial_state.message = if initial_state.status.warn_only {
                "Test mode active. Lock is not enforced.".to_string()
            } else {
                "Waiting for service heartbeat...".to_string()
            };
        }

        let state = Arc::new(Mutex::new(initial_state));
        let _ = UI_STATE.set(state.clone());
        crate::command_pipe::spawn_command_server_thread();
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
        let (style, ex_style, x, y, width, height) = if let Some(state) = UI_STATE.get() {
            let guard = state.lock().unwrap();
            if guard.status.warn_only {
                (WS_OVERLAPPED | WS_VISIBLE, WINDOW_EX_STYLE::default(), 40, 40, WARN_ONLY_WIDTH, WARN_ONLY_HEIGHT)
            } else {
                (
                    WS_VISIBLE,
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                    unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
                    unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
                    unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
                    unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
                )
            }
        } else {
            (
                WS_VISIBLE,
                WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) },
                unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) },
                unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) },
                unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) },
            )
        };

        let hwnd = unsafe {
            CreateWindowExW(
                ex_style,
                PCWSTR(class_name.as_ptr()),
                w!("WinParentalControl"),
                style,
                x,
                y,
                width,
                height,
                None,
                None,
                Some(instance.into()),
                None,
            )
        }
        .map_err(|error| error.to_string())?;

        unsafe {
            let _ = ShowWindow(hwnd, SW_SHOW);
            SetTimer(Some(hwnd), TIMER_ID, 1000, None);
            if let Some(state) = UI_STATE.get() {
                if !state.lock().unwrap().status.warn_only {
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        }

        if let Some(state) = UI_STATE.get() {
            state.lock().unwrap().hwnd = hwnd_to_raw(hwnd);
        }

        let mut message = MSG::default();
        while unsafe { GetMessageW(&mut message, None, 0, 0) }.into() {
            unsafe {
                let _ = TranslateMessage(&message);
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
                    ui.message = if ui.status.mode == DeviceMode::Locked && ui.status.warn_only {
                        "Test mode: lock would be active now, but enforcement is disabled."
                            .to_string()
                    } else if ui.status.mode == DeviceMode::Locked {
                        "Locked. Parent PIN required.".to_string()
                    } else if ui.status.warn_only {
                        format!(
                            "Test mode active. {} remaining until lock resumes.",
                            countdown_text(&ui.status)
                        )
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
        let title_font = create_font(-30, FW_BOLD.0 as i32, FF_SWISS.0 as u32, w!("Segoe UI"));
        let timer_font =
            create_font(-54, FW_BOLD.0 as i32, FF_MODERN.0 as u32, w!("Consolas"));
        let body_font = create_font(-20, FW_NORMAL.0 as i32, FF_SWISS.0 as u32, w!("Segoe UI"));

        let title = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("CONTROL ACTIVE"),
                static_center_style(),
                0,
                0,
                100,
                40,
                Some(hwnd),
                Some(hmenu_from_id(ID_TITLE_LABEL)),
                None,
                None,
            )
        }
        .unwrap();
        let timer = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("00:00"),
                static_center_style(),
                0,
                0,
                100,
                72,
                Some(hwnd),
                Some(hmenu_from_id(ID_TIMER_LABEL)),
                None,
                None,
            )
        }
        .unwrap();
        let hint = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Use the timer as your guide. Extend time or lock instantly."),
                static_center_style(),
                0,
                0,
                100,
                40,
                Some(hwnd),
                Some(hmenu_from_id(ID_HINT_LABEL)),
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
                child_style(WS_BORDER.0 | ES_CENTER as u32 | ES_PASSWORD as u32),
                0,
                0,
                100,
                40,
                Some(hwnd),
                Some(hmenu_from_id(ID_PIN_EDIT)),
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
                child_style(WS_BORDER.0 | ES_CENTER as u32),
                0,
                0,
                100,
                40,
                Some(hwnd),
                Some(hmenu_from_id(ID_DURATION_EDIT)),
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
                child_style(BS_PUSHBUTTON as u32),
                0,
                0,
                100,
                40,
                Some(hwnd),
                Some(hmenu_from_id(ID_UNLOCK_BUTTON)),
                None,
                None,
            )
        }
        .unwrap();
        let message = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Waiting for service sync..."),
                static_center_style(),
                0,
                0,
                100,
                72,
                Some(hwnd),
                Some(hmenu_from_id(ID_MESSAGE_LABEL)),
                None,
                None,
            )
        }
        .unwrap();

        apply_font(title, title_font);
        apply_font(timer, timer_font);
        apply_font(hint, body_font);
        apply_font(pin, body_font);
        apply_font(duration, body_font);
        apply_font(button, body_font);
        apply_font(message, body_font);

        if let Some(state) = UI_STATE.get() {
            let mut guard = state.lock().unwrap();
            guard.hwnd = hwnd_to_raw(hwnd);
            guard.title_hwnd = hwnd_to_raw(title);
            guard.timer_hwnd = hwnd_to_raw(timer);
            guard.hint_hwnd = hwnd_to_raw(hint);
            guard.pin_hwnd = hwnd_to_raw(pin);
            guard.duration_hwnd = hwnd_to_raw(duration);
            guard.button_hwnd = hwnd_to_raw(button);
            guard.message_hwnd = hwnd_to_raw(message);
            guard.title_font = hfont_to_raw(title_font);
            guard.timer_font = hfont_to_raw(timer_font);
            guard.body_font = hfont_to_raw(body_font);
        }
    }

    fn layout_controls(hwnd: HWND) {
        let mut rect = RECT::default();
        unsafe { GetClientRect(hwnd, &mut rect) }.unwrap();
        let width = rect.right - rect.left;
        let height = rect.bottom - rect.top;
        let center_x = width / 2;
        let top = height / 2 - 180;
        let panel_width = 460;
        let left = center_x - panel_width / 2;

        move_control(ID_TITLE_LABEL, left, top, panel_width, 40);
        move_control(ID_TIMER_LABEL, left, top + 46, panel_width, 86);
        move_control(ID_HINT_LABEL, left, top + 136, panel_width, 34);
        move_control(ID_MESSAGE_LABEL, left + 24, top + 176, panel_width - 48, 60);
        move_control(ID_PIN_EDIT, left + 50, top + 252, panel_width - 100, 38);
        move_control(ID_DURATION_EDIT, left + 50, top + 298, panel_width - 100, 38);
        move_control(ID_UNLOCK_BUTTON, left + 50, top + 346, panel_width - 100, 44);
    }

    fn move_control(id: i32, x: i32, y: i32, width: i32, height: i32) {
        if let Some(state) = UI_STATE.get() {
            let hwnd = hwnd_from_raw(state.lock().unwrap().hwnd);
            if let Ok(child) = unsafe { GetDlgItem(Some(hwnd), id) } {
                unsafe {
                    SetWindowPos(child, None, x, y, width, height, SWP_NOACTIVATE)
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
            (hwnd_from_raw(guard.pin_hwnd), hwnd_from_raw(guard.duration_hwnd))
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
        let copied = unsafe { GetWindowTextW(hwnd, &mut buffer) };
        String::from_utf16_lossy(&buffer[..copied as usize])
            .trim()
            .to_string()
    }

    fn refresh_ui() {
        let Some(state) = UI_STATE.get() else {
            return;
        };
        let (
            status,
            message,
            title_hwnd,
            timer_hwnd,
            hint_hwnd,
            message_hwnd,
            hwnd,
            was_visible,
        ) = {
            let guard = state.lock().unwrap();
            (
                guard.status.clone(),
                guard.message.clone(),
                hwnd_from_raw(guard.title_hwnd),
                hwnd_from_raw(guard.timer_hwnd),
                hwnd_from_raw(guard.hint_hwnd),
                hwnd_from_raw(guard.message_hwnd),
                hwnd_from_raw(guard.hwnd),
                guard.is_visible,
            )
        };

        let countdown = countdown_text(&status);
        let title = match status.mode {
            DeviceMode::Locked if status.warn_only => "TEST MODE".to_string(),
            DeviceMode::Locked => "SCREEN LOCKED".to_string(),
            DeviceMode::Unlocked if status.warn_only => "TEST WINDOW ACTIVE".to_string(),
            DeviceMode::Unlocked => "ACCESS GRANTED".to_string(),
        };

        let hint = format!(
            "{}  |  Agent {}  |  Session {}",
            if status.warn_only {
                "Warn-only mode"
            } else {
                "Lock enforced"
            },
            if status.agent_healthy {
                "healthy"
            } else {
                "stale"
            },
            if status.protected_user_logged_in {
                "online"
            } else {
                "offline"
            }
        );
        let timer = match status.mode {
            DeviceMode::Unlocked => countdown.clone(),
            DeviceMode::Locked if status.warn_only => "LOCKED".to_string(),
            DeviceMode::Locked => "00:00".to_string(),
        };
        let detail = match status.mode {
            DeviceMode::Unlocked if status.warn_only => format!(
                "Countdown running live. Lock resumes at {}.",
                status
                    .unlock_expires_at_utc
                    .map(|time| time.with_timezone(&Local).format("%H:%M:%S").to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            DeviceMode::Unlocked => format!(
                "Unlocked until {}. You can still enter a PIN locally if needed.",
                status
                    .unlock_expires_at_utc
                    .map(|time| time.with_timezone(&Local).format("%Y-%m-%d %H:%M:%S").to_string())
                    .unwrap_or_else(|| "unknown".to_string())
            ),
            DeviceMode::Locked if status.warn_only => {
                "Lock would be enforced now, but warn-only mode keeps this as a visible preview."
                    .to_string()
            }
            DeviceMode::Locked => "Parent PIN and duration are required to unlock this session."
                .to_string(),
        };
        let title_wide = wide(&title);
        let timer_wide = wide(&timer);
        let hint_wide = wide(&hint);
        let message_wide = wide(if message.is_empty() { &detail } else { &message });
        unsafe {
            SetWindowTextW(title_hwnd, PCWSTR(title_wide.as_ptr())).unwrap();
            SetWindowTextW(timer_hwnd, PCWSTR(timer_wide.as_ptr())).unwrap();
            SetWindowTextW(hint_hwnd, PCWSTR(hint_wide.as_ptr())).unwrap();
            SetWindowTextW(message_hwnd, PCWSTR(message_wide.as_ptr())).unwrap();
        }

        let mut is_visible = was_visible;
        let should_show = status.mode == DeviceMode::Locked || status.warn_only;
        if should_show {
            if !is_visible {
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                }
                is_visible = true;
            }
            if status.warn_only {
                unsafe {
                    SetWindowPos(
                        hwnd,
                        None,
                        40,
                        40,
                        WARN_ONLY_WIDTH,
                        WARN_ONLY_HEIGHT,
                        SWP_SHOWWINDOW,
                    )
                    .unwrap();
                }
            } else {
                unsafe {
                    SetWindowPos(
                        hwnd,
                        Some(HWND_TOPMOST),
                        0,
                        0,
                        0,
                        0,
                        SWP_NOACTIVATE | SWP_SHOWWINDOW,
                    )
                    .unwrap();
                    let _ = SetForegroundWindow(hwnd);
                }
            }
        } else if was_visible {
            unsafe {
                let _ = ShowWindow(hwnd, SW_HIDE);
            }
            is_visible = false;
        }

        if is_visible != was_visible {
            if let Some(state) = UI_STATE.get() {
                state.lock().unwrap().is_visible = is_visible;
            }
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
