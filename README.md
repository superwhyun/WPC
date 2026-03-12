# WinParentalControl

Rust workspace for a Windows-only parental control agent and service that:

- locks the desktop immediately after login
- allows parent unlock/extend/lock over a local web endpoint
- requires a parent PIN for destructive or unlock actions
- keeps local state in `C:\ProgramData\WinParentalControl\config.json`

## Workspace

- `crates/winpc-core`: shared config, state transitions, DTOs, PIN hashing, DPAPI helpers
- `crates/winpc-service`: Windows Service, localhost HTTP API, IPC server
- `crates/winpc-agent`: child-session lock overlay and local PIN unlock UI
- `scripts/`: installation and setup helpers for Windows

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
  - `POST /api/service/stop`
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

## Notes

- The service listens on `127.0.0.1:46391` by default.
- Remaining time is owned by the service and persisted in `config.json`; the agent only renders the service state and sends heartbeats.
- The service supervisor relaunches the agent for the active console session when heartbeats go stale, and the install script configures Windows Service recovery to restart the service after unexpected termination.
- For intentional maintenance stops, use `scripts/stop-service.ps1` so the service asks for the parent PIN before shutting down.
- The Windows-specific code paths were not executed in this macOS workspace. Only the shared/core and non-Windows build paths were tested here with `cargo test`.
- The current agent implementation uses one topmost window across the full virtual desktop, which covers multi-monitor setups through the virtual screen bounds.
