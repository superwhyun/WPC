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

    use chrono::Utc;
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
                CreateFontW, CLEARTYPE_QUALITY, CLIP_DEFAULT_PRECIS, 
                DEFAULT_CHARSET, DEFAULT_PITCH, DeleteObject, FF_MODERN, FF_SWISS, 
                FONT_CLIP_PRECISION, FONT_OUTPUT_PRECISION, FW_BOLD, FW_NORMAL, 
                GetDC, HBRUSH, HFONT, HDC, OUT_DEFAULT_PRECIS, 
                ReleaseDC, SetBkMode, SetTextColor, TRANSPARENT,
                COLOR_WINDOW,
            },
            System::{
                LibraryLoader::GetModuleHandleW,
                SystemServices::{SS_CENTER, STATIC_STYLES},
                Threading::CreateMutexW,
            },
            UI::WindowsAndMessaging::{
                CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, FindWindowW,
                GetClientRect, GetDlgItem, GetMessageW, GetSystemMetrics, GetWindowRect,
                GetWindowTextLengthW, GetWindowTextW, LoadCursorW, PostMessageW, PostQuitMessage,
                RegisterClassW, SendMessageW, SetForegroundWindow,
                SetTimer, SetWindowPos, SetWindowTextW, ShowWindow, 
                TranslateMessage, BS_PUSHBUTTON, ES_CENTER, ES_PASSWORD, HMENU, HTCAPTION, 
                HWND_TOPMOST, IDC_ARROW, MSG, SM_CXVIRTUALSCREEN, 
                SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN, SM_YVIRTUALSCREEN,
                SWP_FRAMECHANGED, SWP_NOACTIVATE, SWP_SHOWWINDOW, SW_HIDE, SW_SHOW, 
                WINDOW_EX_STYLE, WINDOW_STYLE, WM_APP, WM_CLOSE, WM_COMMAND, WM_CREATE, 
                WM_DESTROY, WM_ERASEBKGND, WM_MOVE, WM_NCHITTEST, WM_SETFONT, 
                WM_SIZE, WM_TIMER, WNDCLASSW, WS_BORDER, WS_CHILD, WS_EX_TOOLWINDOW, 
                WS_EX_TOPMOST, WS_OVERLAPPED, WS_POPUP, WS_VISIBLE,
            },
        },
    };
    use winpc_core::{DeviceMode, DeviceStatus, IpcRequest, IpcResponse, UnlockExpiryAction};

    const PIPE_NAME: &str = r"\\.\pipe\WinParentalControlIpc";
    const WINDOW_CLASS: &str = "WinParentalControlAgentWindow";
    const TIMER_ID: usize = 1;
    const SERVICE_DISCONNECT_FAILURES_BEFORE_EXIT: usize = 2;
    
    const WM_SERVICE_DISCONNECTED: u32 = WM_APP + 1;
    const WM_RESTORE_EXISTING_INSTANCE: u32 = WM_APP + 2;
    const ID_PIN_EDIT: i32 = 2001;
    const ID_DURATION_EDIT: i32 = 2002;
    const ID_UNLOCK_BUTTON: i32 = 2003;
    const ID_TITLE_LABEL: i32 = 2004;
    const ID_MESSAGE_LABEL: i32 = 2005;
    const ID_HINT_LABEL: i32 = 2006;
    const ID_TIMER_LABEL: i32 = 2007;
    const ID_EXPIRY_ACTION_BUTTON: i32 = 2008;
    const ID_TIMER_PIN_EDIT: i32 = 2009;
    const ID_TIMER_EXTEND_BUTTON: i32 = 2010;
    
    // Lock screen dimensions
    const LOCK_CARD_WIDTH: i32 = 420;
    const LOCK_CARD_HEIGHT: i32 = 520;
    
    // Timer widget - compact metallic design
    const TIMER_WIDGET_WIDTH: i32 = 200;
    const TIMER_WIDGET_HEIGHT: i32 = 60;
    const TIMER_WIDGET_MARGIN: i32 = 16;

    static UI_STATE: OnceLock<Arc<Mutex<UiState>>> = OnceLock::new();

    #[derive(Debug, Clone, PartialEq)]
    enum ViewMode {
        FullscreenLock,
        TimerWidget,
        WarnOnly,
    }

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
        expiry_action_hwnd: isize,
        button_hwnd: isize,
        message_hwnd: isize,
        timer_pin_hwnd: isize,
        timer_extend_button_hwnd: isize,
        title_font: isize,
        timer_font: isize,
        body_font: isize,
        is_visible: bool,
        selected_expiry_action: UnlockExpiryAction,
        timer_widget_x: i32,
        timer_widget_y: i32,
        timer_widget_position_set: bool,
        current_view_mode: Option<ViewMode>,
    }

    impl Default for UiState {
        fn default() -> Self {
            Self {
                status: DeviceStatus {
                    mode: DeviceMode::Locked,
                    warn_only: false,
                    unlock_expires_at_utc: None,
                    unlock_expiry_action: None,
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
                expiry_action_hwnd: 0,
                button_hwnd: 0,
                message_hwnd: 0,
                timer_pin_hwnd: 0,
                timer_extend_button_hwnd: 0,
                title_font: 0,
                timer_font: 0,
                body_font: 0,
                is_visible: true,
                selected_expiry_action: UnlockExpiryAction::AppLock,
                timer_widget_x: 0,
                timer_widget_y: 0,
                timer_widget_position_set: false,
                current_view_mode: None,
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

    fn next_expiry_action(action: UnlockExpiryAction) -> UnlockExpiryAction {
        match action {
            UnlockExpiryAction::AppLock => UnlockExpiryAction::WindowsLock,
            UnlockExpiryAction::WindowsLock => UnlockExpiryAction::Shutdown,
            UnlockExpiryAction::Shutdown => UnlockExpiryAction::AppLock,
        }
    }

    fn expiry_action_label(action: UnlockExpiryAction) -> &'static str {
        match action {
            UnlockExpiryAction::AppLock => "App lock",
            UnlockExpiryAction::WindowsLock => "Windows lock",
            UnlockExpiryAction::Shutdown => "Shutdown",
        }
    }

    fn expiry_action_button_text(action: UnlockExpiryAction) -> String {
        format!("On timeout: {}", expiry_action_label(action))
    }

    fn timer_widget_mode(status: &DeviceStatus) -> bool {
        status.mode == DeviceMode::Unlocked
    }

    fn default_timer_widget_position() -> (i32, i32) {
        let left = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let top = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        (
            left + width - TIMER_WIDGET_WIDTH - TIMER_WIDGET_MARGIN,
            top + TIMER_WIDGET_MARGIN,
        )
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

    fn set_control_colors(hwnd: HWND, text_color: u32, bg_transparent: bool) {
        unsafe {
            let hdc = GetDC(Some(hwnd));
            if !hdc.is_invalid() {
                SetTextColor(hdc, windows::Win32::Foundation::COLORREF(text_color));
                if bg_transparent {
                    SetBkMode(hdc, TRANSPARENT);
                }
                ReleaseDC(Some(hwnd), hdc);
            }
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
            restore_existing_instance();
            return Ok(());
        }

        let mut initial_state = UiState::default();
        if let Ok(IpcResponse::State(status) | IpcResponse::Ack(status)) =
            pipe_request(IpcRequest::GetState)
        {
            initial_state.status = status;
            initial_state.selected_expiry_action = initial_state
                .status
                .unlock_expiry_action
                .unwrap_or(UnlockExpiryAction::AppLock);
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
            hbrBackground: HBRUSH((COLOR_WINDOW.0 + 1) as *mut c_void),
            ..Default::default()
        };

        unsafe { RegisterClassW(&class) };
        
        let (style, ex_style, x, y, width, height) = if let Some(state) = UI_STATE.get() {
            let guard = state.lock().unwrap();
            let is_timer_widget = timer_widget_mode(&guard.status);
            if guard.status.warn_only {
                // Warn mode - regular window with border
                (
                    WS_OVERLAPPED | WS_VISIBLE,
                    WINDOW_EX_STYLE::default(),
                    40, 40, 480, 360,
                )
            } else if is_timer_widget {
                // Unlocked: Compact metallic widget, no title bar, no border
                let (default_x, default_y) = default_timer_widget_position();
                (
                    WS_POPUP | WS_VISIBLE,
                    WS_EX_TOPMOST | WS_EX_TOOLWINDOW,
                    default_x, default_y,
                    TIMER_WIDGET_WIDTH, TIMER_WIDGET_HEIGHT,
                )
            } else {
                // Locked: Fullscreen, no decorations
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
                x, y, width, height,
                None, None, Some(instance.into()), None,
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
        thread::spawn(move || {
            let mut consecutive_service_failures = 0usize;
            loop {
                let heartbeat = pipe_request(IpcRequest::Heartbeat);
                let state_response = pipe_request(IpcRequest::GetState);
                let service_reachable = heartbeat.is_ok() || state_response.is_ok();
                if service_reachable {
                    consecutive_service_failures = 0;
                } else {
                    consecutive_service_failures += 1;
                }

                let mut ui = state.lock().unwrap();
                match (heartbeat, state_response) {
                    (Ok(_), Ok(IpcResponse::State(status))) => {
                        ui.status = status;
                        ui.selected_expiry_action = ui
                            .status
                            .unlock_expiry_action
                            .unwrap_or(ui.selected_expiry_action);
                        ui.message = if ui.status.mode == DeviceMode::Locked && ui.status.warn_only
                        {
                            "Test mode: lock would be active".to_string()
                        } else if ui.status.mode == DeviceMode::Locked {
                            "Enter PIN to unlock".to_string()
                        } else if ui.status.warn_only {
                            format!("Test: {} remaining", countdown_text(&ui.status))
                        } else {
                            countdown_text(&ui.status)
                        };
                    }
                    (Ok(IpcResponse::Ack(status)), _) => {
                        ui.status = status;
                        ui.selected_expiry_action = ui
                            .status
                            .unlock_expiry_action
                            .unwrap_or(ui.selected_expiry_action);
                    }
                    (_, Ok(IpcResponse::State(status))) => {
                        ui.status = status;
                        ui.selected_expiry_action = ui
                            .status
                            .unlock_expiry_action
                            .unwrap_or(ui.selected_expiry_action);
                        ui.message = "Service issue - locked".to_string();
                    }
                    (Err(error), _) | (_, Err(error)) => {
                        ui.status.mode = DeviceMode::Locked;
                        ui.message = format!("Service: {error}");
                    }
                    (_, Ok(IpcResponse::Error { message })) => {
                        ui.status.mode = DeviceMode::Locked;
                        ui.message = message;
                    }
                    _ => {}
                }
                drop(ui);

                if consecutive_service_failures >= SERVICE_DISCONNECT_FAILURES_BEFORE_EXIT {
                    request_agent_exit(&state, "service is unavailable");
                    break;
                }

                thread::sleep(Duration::from_secs(3));
            }
        });
    }

    fn request_agent_exit(state: &Arc<Mutex<UiState>>, reason: &str) {
        eprintln!("service disconnected; exiting agent: {reason}");
        let hwnd = {
            let ui = state.lock().unwrap();
            hwnd_from_raw(ui.hwnd)
        };
        if hwnd.0.is_null() {
            std::process::exit(0);
        }

        let _ = unsafe { PostMessageW(Some(hwnd), WM_SERVICE_DISCONNECTED, WPARAM(0), LPARAM(0)) };
    }

    fn restore_existing_instance() {
        let class_name = wide(WINDOW_CLASS);
        if let Ok(hwnd) = unsafe { FindWindowW(PCWSTR(class_name.as_ptr()), PCWSTR::null()) } {
            let _ = unsafe {
                PostMessageW(
                    Some(hwnd),
                    WM_RESTORE_EXISTING_INSTANCE,
                    WPARAM(0),
                    LPARAM(0),
                )
            };
        }
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
            WM_ERASEBKGND => {
                let hdc = HDC(wparam.0 as *mut c_void);
                let mut rect = RECT::default();
                if unsafe { GetClientRect(hwnd, &mut rect) }.is_ok() {
                    let is_timer = UI_STATE
                        .get()
                        .map(|state| timer_widget_mode(&state.lock().unwrap().status))
                        .unwrap_or(false);
                    
                    let color = if is_timer {
                        // Metallic silver-gray for timer widget - brighter
                        0x404050
                    } else {
                        // Dark for lock screen
                        0x0f1115
                    };
                    
                    let brush = unsafe { windows::Win32::Graphics::Gdi::CreateSolidBrush(
                        windows::Win32::Foundation::COLORREF(color)
                    ) };
                    unsafe { windows::Win32::Graphics::Gdi::FillRect(hdc, &rect, brush) };
                    unsafe { let _ = DeleteObject(brush.into()); };
                }
                LRESULT(1)
            }
            WM_SIZE => {
                layout_controls(hwnd);
                LRESULT(0)
            }
            WM_MOVE => {
                remember_timer_widget_position(hwnd);
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
                } else if control_id == ID_EXPIRY_ACTION_BUTTON {
                    cycle_expiry_action();
                } else if control_id == ID_TIMER_EXTEND_BUTTON {
                    handle_extend();
                }
                LRESULT(0)
            }
            WM_SERVICE_DISCONNECTED => {
                let _ = unsafe { DestroyWindow(hwnd) };
                LRESULT(0)
            }
            WM_RESTORE_EXISTING_INSTANCE => {
                refresh_ui();
                unsafe {
                    let _ = ShowWindow(hwnd, SW_SHOW);
                    let _ = SetForegroundWindow(hwnd);
                }
                LRESULT(0)
            }
            WM_NCHITTEST
                if UI_STATE
                    .get()
                    .map(|state| timer_widget_mode(&state.lock().unwrap().status))
                    .unwrap_or(false) =>
            {
                LRESULT(HTCAPTION as isize)
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
        // Clean, modern fonts
        let title_font = create_font(-26, FW_BOLD.0 as i32, FF_SWISS.0 as u32, w!("Segoe UI"));
        let timer_font = create_font(-42, FW_BOLD.0 as i32, FF_MODERN.0 as u32, w!("Consolas"));
        let body_font = create_font(-18, FW_NORMAL.0 as i32, FF_SWISS.0 as u32, w!("Segoe UI"));
        let small_font = create_font(-13, FW_NORMAL.0 as i32, FF_SWISS.0 as u32, w!("Segoe UI"));

        // Title - centered, clean
        let title = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("🔒 Locked"),
                static_center_style(),
                0, 0, 100, 36,
                Some(hwnd),
                Some(hmenu_from_id(ID_TITLE_LABEL)),
                None, None,
            )
        }.unwrap();
        
        // Timer display - big and clear
        let timer = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("00:00"),
                static_center_style(),
                0, 0, 100, 56,
                Some(hwnd),
                Some(hmenu_from_id(ID_TIMER_LABEL)),
                None, None,
            )
        }.unwrap();
        
        // Status hint
        let hint = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!("Enter your PIN to unlock"),
                static_center_style(),
                0, 0, 100, 24,
                Some(hwnd),
                Some(hmenu_from_id(ID_HINT_LABEL)),
                None, None,
            )
        }.unwrap();
        
        // PIN input - clean white box
        let pin = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!(""),
                child_style(WS_BORDER.0 | ES_CENTER as u32 | ES_PASSWORD as u32),
                0, 0, 100, 40,
                Some(hwnd),
                Some(hmenu_from_id(ID_PIN_EDIT)),
                None, None,
            )
        }.unwrap();
        
        // Duration input
        let duration = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!("30"),
                child_style(WS_BORDER.0 | ES_CENTER as u32),
                0, 0, 100, 36,
                Some(hwnd),
                Some(hmenu_from_id(ID_DURATION_EDIT)),
                None, None,
            )
        }.unwrap();
        
        // Expiry action button
        let expiry_action = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                PCWSTR(wide(&expiry_action_button_text(UnlockExpiryAction::AppLock)).as_ptr()),
                child_style(BS_PUSHBUTTON as u32),
                0, 0, 100, 36,
                Some(hwnd),
                Some(hmenu_from_id(ID_EXPIRY_ACTION_BUTTON)),
                None, None,
            )
        }.unwrap();
        
        // Unlock button
        let button = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("Unlock"),
                child_style(BS_PUSHBUTTON as u32),
                0, 0, 100, 42,
                Some(hwnd),
                Some(hmenu_from_id(ID_UNLOCK_BUTTON)),
                None, None,
            )
        }.unwrap();
        
        // Message/status area
        let message = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("STATIC"),
                w!(""),
                static_center_style(),
                0, 0, 100, 40,
                Some(hwnd),
                Some(hmenu_from_id(ID_MESSAGE_LABEL)),
                None, None,
            )
        }.unwrap();

        // Timer widget controls (hidden in lock mode)
        let timer_pin = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("EDIT"),
                w!(""),
                child_style(WS_BORDER.0 | ES_CENTER as u32 | ES_PASSWORD as u32),
                0, 0, 100, 28,
                Some(hwnd),
                Some(hmenu_from_id(ID_TIMER_PIN_EDIT)),
                None, None,
            )
        }.unwrap();
        
        let timer_extend_button = unsafe {
            CreateWindowExW(
                WINDOW_EX_STYLE::default(),
                w!("BUTTON"),
                w!("+30m"),
                child_style(BS_PUSHBUTTON as u32),
                0, 0, 60, 28,
                Some(hwnd),
                Some(hmenu_from_id(ID_TIMER_EXTEND_BUTTON)),
                None, None,
            )
        }.unwrap();

        // Apply fonts
        apply_font(title, title_font);
        apply_font(timer, timer_font);
        apply_font(hint, small_font);
        apply_font(pin, body_font);
        apply_font(duration, body_font);
        apply_font(expiry_action, small_font);
        apply_font(button, body_font);
        apply_font(message, small_font);
        apply_font(timer_pin, small_font);
        apply_font(timer_extend_button, small_font);
        
        // Set colors for dark theme
        set_control_colors(title, 0xffffff, true);
        set_control_colors(timer, 0x4dabf7, true);  // Cyan for timer
        set_control_colors(hint, 0x94a3b8, true);
        set_control_colors(message, 0xffffff, true);  // White for message

        if let Some(state) = UI_STATE.get() {
            let mut guard = state.lock().unwrap();
            guard.hwnd = hwnd_to_raw(hwnd);
            guard.title_hwnd = hwnd_to_raw(title);
            guard.timer_hwnd = hwnd_to_raw(timer);
            guard.hint_hwnd = hwnd_to_raw(hint);
            guard.pin_hwnd = hwnd_to_raw(pin);
            guard.duration_hwnd = hwnd_to_raw(duration);
            guard.expiry_action_hwnd = hwnd_to_raw(expiry_action);
            guard.button_hwnd = hwnd_to_raw(button);
            guard.message_hwnd = hwnd_to_raw(message);
            guard.timer_pin_hwnd = hwnd_to_raw(timer_pin);
            guard.timer_extend_button_hwnd = hwnd_to_raw(timer_extend_button);
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
        let is_unlock = UI_STATE
            .get()
            .map(|state| timer_widget_mode(&state.lock().unwrap().status))
            .unwrap_or(false);

        if is_unlock {
            // === UNLOCK MODE: Compact metallic widget ===
            // No title bar, minimal padding, metallic border effect
            
            // Center the timer text vertically, left-aligned with padding
            move_control(ID_TIMER_LABEL, 12, 8, 90, 44);
            // Small status text on right
            move_control(ID_MESSAGE_LABEL, 100, 22, 92, 20);
            
            // Hide all lock mode controls
            set_control_visible_by_id(ID_TITLE_LABEL, false);
            set_control_visible_by_id(ID_HINT_LABEL, false);
            set_control_visible_by_id(ID_PIN_EDIT, false);
            set_control_visible_by_id(ID_DURATION_EDIT, false);
            set_control_visible_by_id(ID_EXPIRY_ACTION_BUTTON, false);
            set_control_visible_by_id(ID_UNLOCK_BUTTON, false);
            set_control_visible_by_id(ID_TIMER_PIN_EDIT, false);
            set_control_visible_by_id(ID_TIMER_EXTEND_BUTTON, false);
            set_control_visible_by_id(ID_TIMER_LABEL, true);
            set_control_visible_by_id(ID_MESSAGE_LABEL, true);
            return;
        }

        // === LOCK MODE: Clean centered card ===
        let card_width = LOCK_CARD_WIDTH;
        let card_height = LOCK_CARD_HEIGHT;
        let card_x = (width - card_width) / 2;
        let card_y = (height - card_height) / 2;
        
        let content_x = card_x + 40;
        let content_width = card_width - 80;
        
        // Vertical layout with good spacing
        move_control(ID_TITLE_LABEL, content_x, card_y + 40, content_width, 36);
        move_control(ID_TIMER_LABEL, content_x, card_y + 90, content_width, 56);
        move_control(ID_HINT_LABEL, content_x, card_y + 155, content_width, 24);
        move_control(ID_MESSAGE_LABEL, content_x, card_y + 185, content_width, 36);
        move_control(ID_PIN_EDIT, content_x + 40, card_y + 230, content_width - 80, 40);
        
        // Side by side: duration and action
        let half_width = (content_width - 90) / 2;
        move_control(ID_DURATION_EDIT, content_x + 40, card_y + 280, half_width, 36);
        move_control(ID_EXPIRY_ACTION_BUTTON, content_x + 50 + half_width, card_y + 280, half_width, 36);
        
        // Unlock button - prominent
        move_control(ID_UNLOCK_BUTTON, content_x + 40, card_y + 335, content_width - 80, 44);
        
        // Show lock controls, hide timer widget controls
        set_control_visible_by_id(ID_TITLE_LABEL, true);
        set_control_visible_by_id(ID_HINT_LABEL, true);
        set_control_visible_by_id(ID_PIN_EDIT, true);
        set_control_visible_by_id(ID_DURATION_EDIT, true);
        set_control_visible_by_id(ID_EXPIRY_ACTION_BUTTON, true);
        set_control_visible_by_id(ID_UNLOCK_BUTTON, true);
        set_control_visible_by_id(ID_MESSAGE_LABEL, true);
        set_control_visible_by_id(ID_TIMER_LABEL, true);
        set_control_visible_by_id(ID_TIMER_PIN_EDIT, false);
        set_control_visible_by_id(ID_TIMER_EXTEND_BUTTON, false);
    }

    fn move_control(id: i32, x: i32, y: i32, width: i32, height: i32) {
        if let Some(state) = UI_STATE.get() {
            let hwnd = hwnd_from_raw(state.lock().unwrap().hwnd);
            if let Ok(child) = unsafe { GetDlgItem(Some(hwnd), id) } {
                unsafe { SetWindowPos(child, None, x, y, width, height, SWP_NOACTIVATE) }.unwrap();
            }
        }
    }

    fn set_control_visible_by_id(id: i32, visible: bool) {
        if let Some(state) = UI_STATE.get() {
            let hwnd = hwnd_from_raw(state.lock().unwrap().hwnd);
            if let Ok(child) = unsafe { GetDlgItem(Some(hwnd), id) } {
                unsafe {
                    let _ = ShowWindow(child, if visible { SW_SHOW } else { SW_HIDE });
                }
            }
        }
    }

    fn remember_timer_widget_position(hwnd: HWND) {
        let Some(state) = UI_STATE.get() else { return; };
        let mut rect = RECT::default();
        if unsafe { GetWindowRect(hwnd, &mut rect) }.is_err() {
            return;
        }

        let mut guard = state.lock().unwrap();
        if timer_widget_mode(&guard.status) {
            guard.timer_widget_x = rect.left;
            guard.timer_widget_y = rect.top;
            guard.timer_widget_position_set = true;
        }
    }

    fn handle_unlock() {
        let Some(state) = UI_STATE.get() else { return; };
        let (pin_hwnd, duration_hwnd, expiry_action) = {
            let guard = state.lock().unwrap();
            (
                hwnd_from_raw(guard.pin_hwnd),
                hwnd_from_raw(guard.duration_hwnd),
                guard.selected_expiry_action,
            )
        };

        let pin = read_control_text(pin_hwnd);
        let duration = read_control_text(duration_hwnd)
            .parse::<i16>()
            .unwrap_or(30);

        let result = pipe_request(IpcRequest::LocalUnlock {
            pin,
            duration_minutes: duration,
            expiry_action: Some(expiry_action),
        });

        let mut guard = state.lock().unwrap();
        match result {
            Ok(IpcResponse::Ack(status)) => {
                guard.status = status;
                guard.message = "Unlocked".to_string();
                drop(guard);
                refresh_ui();
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

    fn handle_extend() {
        let Some(state) = UI_STATE.get() else { return; };
        let (timer_pin_hwnd, expiry_action) = {
            let guard = state.lock().unwrap();
            (
                hwnd_from_raw(guard.timer_pin_hwnd),
                guard.status.unlock_expiry_action.unwrap_or(guard.selected_expiry_action),
            )
        };

        let pin = read_control_text(timer_pin_hwnd);
        if pin.is_empty() {
            let mut guard = state.lock().unwrap();
            guard.message = "PIN required".to_string();
            return;
        }

        let result = pipe_request(IpcRequest::LocalExtend {
            pin,
            duration_minutes: 30,
            expiry_action: Some(expiry_action),
        });

        let mut guard = state.lock().unwrap();
        match result {
            Ok(IpcResponse::Ack(status)) => {
                guard.status = status;
                guard.message = "+30 min".to_string();
                unsafe {
                    SetWindowTextW(hwnd_from_raw(guard.timer_pin_hwnd), w!("")).ok();
                }
            }
            Ok(IpcResponse::Error { message }) => {
                guard.message = message;
            }
            Err(error) => {
                guard.message = error;
            }
            _ => {}
        }
    }

    fn cycle_expiry_action() {
        let Some(state) = UI_STATE.get() else { return; };
        {
            let mut guard = state.lock().unwrap();
            guard.selected_expiry_action = next_expiry_action(guard.selected_expiry_action);
        }
        refresh_ui();
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
        let Some(state) = UI_STATE.get() else { return; };

        {
            let mut guard = state.lock().unwrap();
            if timer_widget_mode(&guard.status) && !guard.timer_widget_position_set {
                let (x, y) = default_timer_widget_position();
                guard.timer_widget_x = x;
                guard.timer_widget_y = y;
                guard.timer_widget_position_set = true;
            }
        }

        let (
            status,
            message,
            title_hwnd,
            timer_hwnd,
            hint_hwnd,
            _pin_hwnd,
            _duration_hwnd,
            expiry_action_hwnd,
            _button_hwnd,
            message_hwnd,
            _timer_pin_hwnd,
            _timer_extend_button_hwnd,
            hwnd,
            was_visible,
            selected_expiry_action,
            timer_widget_x,
            timer_widget_y,
        ) = {
            let guard = state.lock().unwrap();
            (
                guard.status.clone(),
                guard.message.clone(),
                hwnd_from_raw(guard.title_hwnd),
                hwnd_from_raw(guard.timer_hwnd),
                hwnd_from_raw(guard.hint_hwnd),
                hwnd_from_raw(guard.pin_hwnd),
                hwnd_from_raw(guard.duration_hwnd),
                hwnd_from_raw(guard.expiry_action_hwnd),
                hwnd_from_raw(guard.button_hwnd),
                hwnd_from_raw(guard.message_hwnd),
                hwnd_from_raw(guard.timer_pin_hwnd),
                hwnd_from_raw(guard.timer_extend_button_hwnd),
                hwnd_from_raw(guard.hwnd),
                guard.is_visible,
                guard.selected_expiry_action,
                guard.timer_widget_x,
                guard.timer_widget_y,
            )
        };

        let is_unlock = timer_widget_mode(&status);
        let countdown = countdown_text(&status);
        let selected_expiry_action = status
            .unlock_expiry_action
            .unwrap_or(selected_expiry_action);
        
        let title = if status.mode == DeviceMode::Locked {
            if status.warn_only { "🔓 Test Mode" } else { "🔒 Locked" }
        } else {
            "✓ Unlocked"
        };

        let hint = if status.mode == DeviceMode::Locked {
            "Enter PIN to unlock"
        } else {
            "Time remaining"
        };
        
        let timer_text = if is_unlock { countdown.clone() } else { "--:--".to_string() };
        let timeout_action = format!("On timeout: {}", expiry_action_label(selected_expiry_action));
        
        let message_text = if is_unlock {
            countdown
        } else if message.is_empty() {
            timeout_action
        } else {
            message
        };

        unsafe {
            SetWindowTextW(title_hwnd, PCWSTR(wide(title).as_ptr())).unwrap();
            SetWindowTextW(timer_hwnd, PCWSTR(wide(&timer_text).as_ptr())).unwrap();
            SetWindowTextW(hint_hwnd, PCWSTR(wide(hint).as_ptr())).unwrap();
            SetWindowTextW(expiry_action_hwnd, PCWSTR(wide(&expiry_action_button_text(selected_expiry_action)).as_ptr())).unwrap();
            SetWindowTextW(message_hwnd, PCWSTR(wide(&message_text).as_ptr())).unwrap();
        }

        let mut is_visible = was_visible;
        if !is_visible {
            unsafe { let _ = ShowWindow(hwnd, SW_SHOW); }
            is_visible = true;
        }

        // Determine target view mode
        let target_mode = if is_unlock {
            ViewMode::TimerWidget
        } else if status.warn_only {
            ViewMode::WarnOnly
        } else {
            ViewMode::FullscreenLock
        };

        // Check if we need to reposition/resize
        let should_reposition = if target_mode == ViewMode::FullscreenLock {
            true
        } else if let Some(state) = UI_STATE.get() {
            let guard = state.lock().unwrap();
            guard.current_view_mode.as_ref() != Some(&target_mode)
        } else {
            false
        };

        if should_reposition {
            match target_mode {
                ViewMode::TimerWidget => {
                    unsafe {
                        SetWindowPos(
                            hwnd, Some(HWND_TOPMOST),
                            timer_widget_x, timer_widget_y,
                            TIMER_WIDGET_WIDTH, TIMER_WIDGET_HEIGHT,
                            SWP_NOACTIVATE | SWP_SHOWWINDOW | SWP_FRAMECHANGED,
                        ).unwrap();
                    }
                }
                ViewMode::WarnOnly => {
                    unsafe {
                        SetWindowPos(
                            hwnd, None, 40, 40, 480, 360,
                            SWP_SHOWWINDOW,
                        ).unwrap();
                    }
                }
                ViewMode::FullscreenLock => {
                    unsafe {
                        SetWindowPos(
                            hwnd, Some(HWND_TOPMOST),
                            GetSystemMetrics(SM_XVIRTUALSCREEN),
                            GetSystemMetrics(SM_YVIRTUALSCREEN),
                            GetSystemMetrics(SM_CXVIRTUALSCREEN),
                            GetSystemMetrics(SM_CYVIRTUALSCREEN),
                            SWP_NOACTIVATE | SWP_SHOWWINDOW,
                        ).unwrap();
                        let _ = SetForegroundWindow(hwnd);
                    }
                }
            }

            if let Some(state) = UI_STATE.get() {
                state.lock().unwrap().current_view_mode = Some(target_mode);
            }
            
            // Force re-layout after mode change
            layout_controls(hwnd);
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
