# WinParentalControl

Rust workspace for a Windows-only parental control agent and service that:

- locks the child desktop immediately after login
- allows parent unlock/extend/lock over a Tailscale-only web endpoint
- requires a parent PIN for destructive or unlock actions
- keeps local state in `C:\ProgramData\WinParentalControl\config.json`

## Workspace

- `crates/winpc-core`: shared config, state transitions, DTOs, PIN hashing, DPAPI helpers
- `crates/winpc-service`: Windows Service, localhost HTTP API, Tailscale identity checks, IPC server
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
  - `GET /healthz`
- Tailscale identity allowlist via `Tailscale-User-Login`
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

In another child-user session, run:

```powershell
.\target\release\winpc-agent.exe
```

## Install on Windows

1. Build the release binaries on Windows.
2. Run [scripts/install-agent.ps1](/Users/whyun/workspace/WinParentalControl/scripts/install-agent.ps1).
3. Run [scripts/install-service.ps1](/Users/whyun/workspace/WinParentalControl/scripts/install-service.ps1).
4. Run [scripts/configure-child-user.ps1](/Users/whyun/workspace/WinParentalControl/scripts/configure-child-user.ps1) with:
   - `-ChildUser <local-user-name>`
   - `-Pin <parent-pin>`
   - `-AllowedLogins parent1@example.com,parent2@example.com`
5. Run [scripts/configure-tailscale.ps1](/Users/whyun/workspace/WinParentalControl/scripts/configure-tailscale.ps1).

Example:

```powershell
.\scripts\configure-child-user.ps1 `
  -ChildUser kid `
  -Pin 4321 `
  -AllowedLogins parent@example.com
```

## Notes

- The service listens only on `127.0.0.1:46391`. Tailscale Serve should be the only remote entrypoint.
- The Windows-specific code paths were not executed in this macOS workspace. Only the shared/core and non-Windows build paths were tested here with `cargo test`.
- The current agent implementation uses one topmost window across the full virtual desktop, which covers multi-monitor setups through the virtual screen bounds.
