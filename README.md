# WinParentalControl

Rust workspace for a Windows-only parental control agent and service that:

- locks the desktop immediately after login
- allows parent unlock/extend/lock over a local web endpoint
- requires a parent PIN for destructive or unlock actions
- keeps local state in `C:\ProgramData\WinParentalControl\config.json`

## Workspace

- `crates/winpc-core`: shared config, state transitions, DTOs, PIN hashing, DPAPI helpers
- `crates/winpc-service`: Windows Service, HTTP API (port 46391), IPC server
- `crates/winpc-agent`: child-session lock overlay and local PIN unlock UI
- `scripts/`: PowerShell installation and setup helpers
- `WPC_Installer.iss`: Inno Setup script for automated installer generation

## Current shape

This repository already includes:

- time-based lock state persisted in config
- Argon2 PIN hashing with DPAPI protection on Windows
- localhost-only Axum API with:
  - `GET /api/device/status`
  - `POST /api/auth/pin`
  - `POST /api/device/unlock`
  - `POST /api/device/extend`
  - `POST /api/device/lock`
  - `POST /api/device/windows-lock`
  - `POST /api/device/shutdown`
  - `GET /healthz`
- named-pipe IPC contract for `GetState`, `Heartbeat`, and `LocalUnlock`
- Windows-only service/agent entrypoints with non-Windows stubs so the workspace still builds on macOS

## Build

On Windows, install:

- Rust stable
- MSVC Build Tools
- Windows SDK
- VS Code or another editor

Build from a Windows shell:

```powershell
cargo build --release
```

For local debugging without SCM, run the service in console mode:

```powershell
.\target\release\winpc-service.exe --console
```

In another interactive user session, run:

```powershell
.\target\release\winpc-agent.exe
```

## Install on Windows

### Option 1: Installer (Recommended)

1. Build the release binaries:
   ```powershell
   cargo build --release
   ```

2. Create the installer using Inno Setup:
   - Install [Inno Setup](https://jrsoftware.org/isinfo.php)
   - Compile `WPC_Installer.iss`
   - Run the generated `WPC_Setup.exe`

**The installer automatically:**
- ✅ Installs service and agent to `C:\Program Files\WinParentalControl`
- ✅ Registers Windows Service with auto-start on boot
- ✅ Configures agent to auto-start for all users on login
- ✅ Opens Windows Firewall for port 46391 (TCP inbound)
- ✅ Sets up service recovery to auto-restart on crash (5s/15s/30s delays)
- ✅ Initializes default configuration with PIN: `0000`
- ✅ Starts the service immediately
- ✅ Clean uninstall removes all files, service, firewall rules, and startup entries

### Option 2: Manual Installation (PowerShell Scripts)

1. Build the release binaries on Windows.
2. Run `scripts/install-agent.ps1`.
3. Run `scripts/install-service.ps1`.
4. Run `scripts/configure-all-users.ps1` with:
   - `-Pin <parent-pin>`

Example:

```powershell
.\scripts\configure-all-users.ps1 `
  -Pin 4321
```

## PIN Management

### Changing PIN

You can change your PIN through the web interface:
1. Navigate to `http://localhost:46391` (or from another device on the network)
2. Go to the "Security Settings" section
3. Enter your current PIN, new PIN, and confirm the new PIN
4. Click "Change PIN"

### PIN Recovery (If You Forgot Your PIN)

If you forget your PIN, you have several recovery options:

#### Option 1: Using Command Line (Recommended)
Run as Administrator:
```powershell
# Stop the service first
sc stop WinParentalControlService

# Reset PIN to a new value
cd "C:\Program Files\WinParentalControl"
.\winpc-service.exe --reset-pin --pin YOUR_NEW_PIN

# Start the service
sc start WinParentalControlService
```

#### Option 2: Delete Configuration File
Run as Administrator:
```powershell
# Stop the service
sc stop WinParentalControlService

# Delete the config file (will reset to default PIN: 0000)
del "C:\ProgramData\WinParentalControl\config.json"

# Start the service
sc start WinParentalControlService
```

After recovery, the default PIN will be `0000` (Option 2) or your specified new PIN (Option 1).

## Notes

- **Network**: The service listens on `0.0.0.0:46391` and is accessible from the local network. The installer automatically creates a Windows Firewall rule to allow TCP port 46391 inbound.
- **State Management**: Remaining time is owned by the service and persisted in `config.json`; the agent only renders the service state and sends heartbeats.
- **Unlock Actions**: Parent unlock/extend requests can choose the timeout follow-up action: `app_lock`, `windows_lock`, or `shutdown`.
- **Immediate Controls**: Parent controls can also immediately apply `App Lock`, `Windows Lock`, or `Windows Shutdown` without waiting for the timer.
- **Auto-Recovery**:
  - The service supervisor relaunches the agent for the active console session when heartbeats go stale.
  - Windows Service recovery automatically restarts the service after crashes (5s → 15s → 30s delays).
  - If the service process is killed (e.g., via Task Manager), Windows will automatically restart it.
- **Auto-Start**:
  - The service starts automatically on system boot (Windows Service with `start=auto`).
  - The agent starts automatically for all users on login (via Common Startup folder).
- **Testing**: The Windows-specific code paths were not executed in this macOS workspace. Only the shared/core and non-Windows build paths were tested here with `cargo test`.
- **Multi-Monitor**: The current agent implementation uses one topmost window across the full virtual desktop, which covers multi-monitor setups through the virtual screen bounds.
