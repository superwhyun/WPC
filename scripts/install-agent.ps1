param(
    [string]$BuildDir = (Join-Path $PSScriptRoot "..\target\release"),
    [string]$InstallDir = (Join-Path $env:ProgramFiles "WinParentalControl"),
    [string]$DataDir = (Join-Path $env:ProgramData "WinParentalControl")
)

$ErrorActionPreference = "Stop"

if (-not (Test-Path $BuildDir)) {
    throw "Build output not found: $BuildDir"
}

New-Item -ItemType Directory -Force -Path $InstallDir | Out-Null
New-Item -ItemType Directory -Force -Path $DataDir | Out-Null

Copy-Item (Join-Path $BuildDir "winpc-agent.exe") $InstallDir -Force

icacls $DataDir /inheritance:r | Out-Null
icacls $DataDir /grant:r "SYSTEM:(OI)(CI)F" | Out-Null
icacls $DataDir /grant:r "Administrators:(OI)(CI)F" | Out-Null

Write-Host "Agent installed to $InstallDir"
Write-Host "Protected data directory prepared at $DataDir"
