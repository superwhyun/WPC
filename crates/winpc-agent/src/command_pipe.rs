#[cfg(windows)]
mod imp {
    use std::{io, mem::size_of, slice, thread};

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use png::{BitDepth, ColorType, Encoder};
    use tokio::{
        io::{AsyncBufReadExt, AsyncWriteExt, BufReader},
        net::windows::named_pipe::{NamedPipeServer, ServerOptions},
        runtime::Builder,
    };
    use windows::Win32::{
        Foundation::HWND,
        Graphics::Gdi::{
            BitBlt, CreateCompatibleDC, CreateDIBSection, DeleteDC, DeleteObject, GetDC, ReleaseDC,
            SelectObject, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, CAPTUREBLT, DIB_RGB_COLORS,
            HGDIOBJ, ROP_CODE, SRCCOPY,
        },
        UI::WindowsAndMessaging::{
            GetSystemMetrics, SM_CXVIRTUALSCREEN, SM_CYVIRTUALSCREEN, SM_XVIRTUALSCREEN,
            SM_YVIRTUALSCREEN,
        },
    };
    use winpc_core::{AgentCommandRequest, AgentCommandResponse, AGENT_COMMAND_PIPE_NAME};

    pub fn spawn_command_server_thread() {
        thread::spawn(|| {
            let runtime = Builder::new_current_thread()
                .enable_all()
                .build()
                .expect("failed to build agent command runtime");
            if let Err(error) = runtime.block_on(run_command_server()) {
                eprintln!("agent command server failed: {error}");
            }
        });
    }

    async fn run_command_server() -> Result<(), String> {
        loop {
            let server = ServerOptions::new()
                .create(AGENT_COMMAND_PIPE_NAME)
                .map_err(|error| error.to_string())?;
            server.connect().await.map_err(|error| error.to_string())?;
            if let Err(error) = handle_client(server).await {
                eprintln!("agent command client failed: {error}");
            }
        }
    }

    async fn handle_client(pipe: NamedPipeServer) -> Result<(), String> {
        let (read_half, mut write_half) = tokio::io::split(pipe);
        let mut reader = BufReader::new(read_half);
        let mut line = String::new();
        reader
            .read_line(&mut line)
            .await
            .map_err(|error| error.to_string())?;
        if line.trim().is_empty() {
            return Ok(());
        }

        let request: AgentCommandRequest =
            serde_json::from_str(&line).map_err(|error| error.to_string())?;
        let response = match request {
            AgentCommandRequest::CaptureSnapshot => match capture_snapshot_png() {
                Ok(png) => AgentCommandResponse::Snapshot {
                    png_base64: STANDARD.encode(png),
                },
                Err(message) => AgentCommandResponse::Error { message },
            },
        };

        write_half
            .write_all(format!("{}\n", serde_json::to_string(&response).unwrap()).as_bytes())
            .await
            .map_err(|error| error.to_string())?;
        write_half
            .flush()
            .await
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    fn capture_snapshot_png() -> Result<Vec<u8>, String> {
        let x = unsafe { GetSystemMetrics(SM_XVIRTUALSCREEN) };
        let y = unsafe { GetSystemMetrics(SM_YVIRTUALSCREEN) };
        let width = unsafe { GetSystemMetrics(SM_CXVIRTUALSCREEN) };
        let height = unsafe { GetSystemMetrics(SM_CYVIRTUALSCREEN) };

        if width <= 0 || height <= 0 {
            return Err("no active desktop available for capture".to_string());
        }

        unsafe {
            let screen_dc = GetDC(HWND::default());
            if screen_dc.0 == 0 {
                return Err(last_os_error_message());
            }

            let memory_dc = CreateCompatibleDC(screen_dc);
            if memory_dc.0 == 0 {
                let _ = ReleaseDC(HWND::default(), screen_dc);
                return Err(last_os_error_message());
            }

            let mut bitmap_info = BITMAPINFO {
                bmiHeader: BITMAPINFOHEADER {
                    biSize: size_of::<BITMAPINFOHEADER>() as u32,
                    biWidth: width,
                    biHeight: -height,
                    biPlanes: 1,
                    biBitCount: 32,
                    biCompression: BI_RGB.0,
                    ..Default::default()
                },
                ..Default::default()
            };
            let mut raw_bits = std::ptr::null_mut();
            let bitmap = CreateDIBSection(
                screen_dc,
                &bitmap_info,
                DIB_RGB_COLORS,
                &mut raw_bits,
                None,
                0,
            );
            if bitmap.0 == 0 || raw_bits.is_null() {
                let _ = DeleteDC(memory_dc);
                let _ = ReleaseDC(HWND::default(), screen_dc);
                return Err(last_os_error_message());
            }

            let previous = SelectObject(memory_dc, bitmap.into());
            if previous.0 == 0 {
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(memory_dc);
                let _ = ReleaseDC(HWND::default(), screen_dc);
                return Err(last_os_error_message());
            }

            let raster = ROP_CODE(SRCCOPY.0 | CAPTUREBLT.0);
            let copied = BitBlt(memory_dc, 0, 0, width, height, screen_dc, x, y, raster).as_bool();
            if !copied {
                let _ = SelectObject(memory_dc, previous);
                let _ = DeleteObject(bitmap.into());
                let _ = DeleteDC(memory_dc);
                let _ = ReleaseDC(HWND::default(), screen_dc);
                return Err(last_os_error_message());
            }

            let pixel_len = width as usize * height as usize * 4;
            let bgra = slice::from_raw_parts(raw_bits.cast::<u8>(), pixel_len);
            let mut rgba = vec![0u8; pixel_len];
            for (src, dst) in bgra.chunks_exact(4).zip(rgba.chunks_exact_mut(4)) {
                dst[0] = src[2];
                dst[1] = src[1];
                dst[2] = src[0];
                dst[3] = 255;
            }

            let _ = SelectObject(memory_dc, previous);
            let _ = DeleteObject(bitmap.into());
            let _ = DeleteDC(memory_dc);
            let _ = ReleaseDC(HWND::default(), screen_dc);

            encode_png(width as u32, height as u32, &rgba)
        }
    }

    fn encode_png(width: u32, height: u32, rgba: &[u8]) -> Result<Vec<u8>, String> {
        let mut png = Vec::new();
        let mut encoder = Encoder::new(&mut png, width, height);
        encoder.set_color(ColorType::Rgba);
        encoder.set_depth(BitDepth::Eight);
        let mut writer = encoder.write_header().map_err(|error| error.to_string())?;
        writer
            .write_image_data(rgba)
            .map_err(|error| error.to_string())?;
        drop(writer);
        Ok(png)
    }

    fn last_os_error_message() -> String {
        io::Error::last_os_error().to_string()
    }
}

#[cfg(not(windows))]
mod imp {
    pub fn spawn_command_server_thread() {}
}

pub use imp::spawn_command_server_thread;
