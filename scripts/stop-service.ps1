param(
    [string]$ServiceUrl = "http://127.0.0.1:46391"
)

$ErrorActionPreference = "Stop"

$securePin = Read-Host "Parent PIN" -AsSecureString
$pinPtr = [Runtime.InteropServices.Marshal]::SecureStringToBSTR($securePin)

try {
    $pin = [Runtime.InteropServices.Marshal]::PtrToStringBSTR($pinPtr)
    Invoke-RestMethod `
        -Method Post `
        -Uri "$ServiceUrl/api/service/stop" `
        -ContentType "application/json" `
        -Body (@{ pin = $pin } | ConvertTo-Json) | Out-Null
}
finally {
    if ($pinPtr -ne [IntPtr]::Zero) {
        [Runtime.InteropServices.Marshal]::ZeroFreeBSTR($pinPtr)
    }
}

Write-Host "Service stop requested."
