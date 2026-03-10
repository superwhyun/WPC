param(
    [int]$LocalPort = 46391,
    [int]$HttpsPort = 443
)

$ErrorActionPreference = "Stop"

tailscale serve --bg --https=$HttpsPort ("http://127.0.0.1:{0}" -f $LocalPort)
tailscale serve status
