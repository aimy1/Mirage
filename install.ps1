# Windows PowerShell One-Click Installer for Mirage TUI Audio Visualizer
# Requires ExecutionPolicy to allow running scripts (Set-ExecutionPolicy Bypass -Scope Process)

Write-Host "🌌 Welcome to Mirage Visualizer One-Click Installer" -ForegroundColor Cyan
Write-Host "Checking system environment..." -ForegroundColor Yellow

# 1. 检查 Rust 编译环境
$cargoCheck = Get-Command cargo -ErrorAction SilentlyContinue
if (-not $cargoCheck) {
    Write-Host "Rust/Cargo environment not found. Downloading rustup..." -ForegroundColor Red
    $url = "https://win.rustup.rs/x86_64"
    $output = "$env:TEMP\rustup-init.exe"
    
    Write-Host "Downloading rustup-init.exe from $url ..." -ForegroundColor Yellow
    Invoke-WebRequest -Uri $url -OutFile $output
    
    Write-Host "Starting Rust installer. Please follow instructions in the popup console..." -ForegroundColor Yellow
    Start-Process -FilePath $output -Args "-y" -Wait
    
    # 刷新当前进程的 PATH 环境变量以确保立即生效
    $env:PATH += ";$env:USERPROFILE\.cargo\bin"
} else {
    Write-Host "Rust environment detected: $(cargo --version)" -ForegroundColor Green
}

# 2. 编译并全局安装 Mirage
Write-Host "Building and installing Mirage..." -ForegroundColor Green
cargo install --path .

Write-Host "Mirage successfully installed! You can now launch it by typing 'mirage' in any terminal." -ForegroundColor Green
