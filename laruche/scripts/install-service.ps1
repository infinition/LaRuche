# LaRuche Windows Service Installer
# Run as Administrator
# Usage: .\install-service.ps1 [-Uninstall]

param(
    [switch]$Uninstall
)

$ServiceName = "LaRuche"
$DisplayName = "LaRuche AI Agent"
$Description = "LaRuche Essaim - Networked Edge AI Agent with Miel Protocol"
$ExePath = Join-Path $PSScriptRoot "..\target\release\laruche-node.exe"
$WorkDir = Split-Path $PSScriptRoot

if ($Uninstall) {
    Write-Host "Uninstalling $ServiceName service..."
    sc.exe stop $ServiceName 2>$null
    sc.exe delete $ServiceName
    Write-Host "Service removed."
    exit 0
}

# Build release binary
Write-Host "Building release binary..."
Push-Location $WorkDir
cargo build --release -p laruche-node
Pop-Location

if (-not (Test-Path $ExePath)) {
    Write-Host "ERROR: Binary not found at $ExePath"
    exit 1
}

# Create Windows service using sc.exe
Write-Host "Installing $ServiceName service..."
sc.exe create $ServiceName binPath= "$ExePath" DisplayName= "$DisplayName" start= auto
sc.exe description $ServiceName "$Description"
sc.exe start $ServiceName

Write-Host ""
Write-Host "Service '$ServiceName' installed and started."
Write-Host "Dashboard: http://localhost:8419/dashboard"
Write-Host "Chatbot:   http://localhost:8419/chat"
Write-Host ""
Write-Host "To stop:   sc.exe stop $ServiceName"
Write-Host "To remove: .\install-service.ps1 -Uninstall"
