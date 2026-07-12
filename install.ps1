Write-Host "=== Video Clipper Bootstrapper & Dependency Installer (Windows) ===" -ForegroundColor Cyan

# 1. Check for Git
if (!(Get-Command git -ErrorAction SilentlyContinue)) {
    Write-Host "Git is missing. Installing via winget..." -ForegroundColor Yellow
    winget install --id Git.Git -e --source winget
    # Refresh PATH
    $env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")
} else {
    Write-Host "Git is already installed." -ForegroundColor Green
}

# 2. Check for Winget / Chocolatey
if (!(Get-Command winget -ErrorAction SilentlyContinue)) {
    Write-Error "winget is missing. Please install App Installer from Microsoft Store to continue."
    exit
}

# 3. Check/Install ADB, FFmpeg, Node.js, and Rust
if (!(Get-Command adb -ErrorAction SilentlyContinue)) {
    Write-Host "Installing ADB..." -ForegroundColor Yellow
    winget install --id Google.AdkPlatformTools -e --source winget
} else {
    Write-Host "ADB is already installed." -ForegroundColor Green
}

if (!(Get-Command ffmpeg -ErrorAction SilentlyContinue)) {
    Write-Host "Installing FFmpeg..." -ForegroundColor Yellow
    winget install --id Gyan.FFmpeg -e --source winget
} else {
    Write-Host "FFmpeg is already installed." -ForegroundColor Green
}

if (!(Get-Command node -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Node.js..." -ForegroundColor Yellow
    winget install --id OpenJS.NodeJS.LTS -e --source winget
} else {
    Write-Host "Node.js is already installed." -ForegroundColor Green
}

if (!(Get-Command rustc -ErrorAction SilentlyContinue)) {
    Write-Host "Installing Rust Compiler..." -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --source winget
} else {
    Write-Host "Rust is already installed." -ForegroundColor Green
}

# Refresh PATH again
$env:Path = [System.Environment]::GetEnvironmentVariable("Path","Machine") + ";" + [System.Environment]::GetEnvironmentVariable("Path","User")

# 4. Clone Repository
Write-Host "Cloning Video Clipper repository..." -ForegroundColor Cyan
if (Test-Path "video-clipper") {
    Write-Host "Folder 'video-clipper' already exists. Pulling latest updates..." -ForegroundColor Yellow
    Set-Location video-clipper
    git pull
} else {
    git clone https://github.com/bassemsab/video-clipper.git
    Set-Location video-clipper
}

# 5. Install npm packages
Write-Host "Installing application dependencies..." -ForegroundColor Cyan
npm install

Write-Host "====================================================" -ForegroundColor Green
Write-Host "🎉 Setup Completed Successfully!" -ForegroundColor Green
Write-Host "To start the application in development mode, run:" -ForegroundColor Green
Write-Host "  cd video-clipper" -ForegroundColor Green
Write-Host "  npm run tauri dev" -ForegroundColor Green
Write-Host "====================================================" -ForegroundColor Green
