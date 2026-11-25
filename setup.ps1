param(
    [switch]$EmulatorDeps
)

$ErrorActionPreference = "Continue"
Write-Host "=== Setup Lingkungan KarirAI untuk Windows 10/11 ===" -ForegroundColor Cyan

if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
    Write-Host "[1] Memasang Rustup (Toolchain Rust) via winget..." -ForegroundColor Yellow
    winget install --id Rustlang.Rustup -e --accept-source-agreements --accept-package-agreements
} else {
    Write-Host "[OK] Rust & Cargo sudah terpasang: $(cargo --version)" -ForegroundColor Green
}

if ($EmulatorDeps) {
    if (-not (Get-Command java -ErrorAction SilentlyContinue)) {
        Write-Host "[2] Memasang OpenJDK 21 (syarat Firebase Firestore Emulator)..." -ForegroundColor Yellow
        winget install --id Microsoft.OpenJDK.21 -e --accept-source-agreements --accept-package-agreements
    }
    if (-not (Get-Command npm -ErrorAction SilentlyContinue)) {
        Write-Host "[3] Memasang Node.js LTS (syarat Firebase CLI)..." -ForegroundColor Yellow
        winget install --id OpenJS.NodeJS.LTS -e --accept-source-agreements --accept-package-agreements
    }
    if (-not (Get-Command firebase -ErrorAction SilentlyContinue)) {
        Write-Host "[4] Memasang firebase-tools global via npm..." -ForegroundColor Yellow
        npm install -g firebase-tools
    }
}

Write-Host "`n[SELESAI] Anda siap menjalankan aplikasi dengan klik dua kali run.bat!" -ForegroundColor Green
