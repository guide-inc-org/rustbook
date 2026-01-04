# guidebook installer for Windows
# Usage: irm https://raw.githubusercontent.com/guide-inc-org/guidebook/main/install.ps1 | iex

$ErrorActionPreference = "Stop"

$repo = "guide-inc-org/guidebook"
$artifact = "guidebook-windows-x86_64.zip"
$url = "https://github.com/$repo/releases/latest/download/$artifact"
$installDir = "$env:USERPROFILE\.guidebook\bin"

Write-Host "Downloading $artifact..."

# Create install directory
if (!(Test-Path $installDir)) {
    New-Item -ItemType Directory -Path $installDir -Force | Out-Null
}

# Download
$zipPath = "$env:TEMP\guidebook.zip"
Invoke-WebRequest -Uri $url -OutFile $zipPath

# Extract
Expand-Archive -Path $zipPath -DestinationPath $env:TEMP\guidebook -Force
Move-Item -Path "$env:TEMP\guidebook\guidebook.exe" -Destination "$installDir\guidebook.exe" -Force

# Clean up
Remove-Item $zipPath -Force
Remove-Item "$env:TEMP\guidebook" -Recurse -Force

# Add to PATH if not already there
$userPath = [Environment]::GetEnvironmentVariable("Path", "User")
if ($userPath -notlike "*$installDir*") {
    [Environment]::SetEnvironmentVariable("Path", "$userPath;$installDir", "User")
    Write-Host ""
    Write-Host "Added $installDir to PATH"
    Write-Host "Please restart your terminal for the changes to take effect."
}

Write-Host ""
Write-Host "guidebook installed successfully!"
Write-Host "Run 'guidebook --help' to get started."
