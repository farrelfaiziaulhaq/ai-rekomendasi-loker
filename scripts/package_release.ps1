param(
    [string]$Version = "v1.0.0"
)

$ErrorActionPreference = "Stop"
$Root = Split-Path -Parent $PSScriptRoot
Set-Location $Root

Write-Host "=== Membangun Paket Rilis Windows ($Version) ===" -ForegroundColor Cyan
cargo build --release

$PkgName = "karir-ai-$Version-windows-x86_64"
$DistDir = Join-Path $Root "dist"
$StageDir = Join-Path $DistDir $PkgName

if (Test-Path $StageDir) { Remove-Item -Recurse -Force $StageDir }
New-Item -ItemType Directory -Force -Path $StageDir | Out-Null
New-Item -ItemType Directory -Force -Path (Join-Path $StageDir "data\processed") | Out-Null

Copy-Item (Join-Path $Root "target\release\karir-ai.exe") $StageDir
Copy-Item -Recurse (Join-Path $Root "static") (Join-Path $StageDir "static")
Copy-Item (Join-Path $Root "data\processed\*") (Join-Path $StageDir "data\processed\")
Copy-Item (Join-Path $Root "firebase.json") $StageDir
Copy-Item (Join-Path $Root ".firebaserc") $StageDir
Copy-Item (Join-Path $Root "firestore.rules") $StageDir
Copy-Item (Join-Path $Root ".env.example") $StageDir
Copy-Item (Join-Path $Root "README.md") $StageDir

# Buat peluncur 1-klik paling simpel untuk teman kelompok di Windows (Tanpa Install Rust!)
@"
@echo off
title KarirAI - Sistem Rekomendasi Lowongan Kerja (Siap Pakai Windows)
cd /d "%~dp0"
echo Membuka aplikasi KarirAI di http://127.0.0.1:3000 ...
start "" "http://127.0.0.1:3000"
"%~dp0karir-ai.exe"
pause
"@ | Set-Content -Encoding ASCII (Join-Path $StageDir "start.bat")

@"
@echo off
title KarirAI + Firebase Emulator (:4040)
cd /d "%~dp0"
set FIRESTORE_EMULATOR_HOST=127.0.0.1:8088
set FIREBASE_AUTH_EMULATOR_HOST=127.0.0.1:9098
set FIREBASE_PROJECT_ID=demo-karir-ai
start "Firebase Emulator" cmd /c "firebase emulators:start --project demo-karir-ai --only auth,firestore"
timeout /t 5 /nobreak >nul
start "" "http://127.0.0.1:4040"
start "" "http://127.0.0.1:3000"
"%~dp0karir-ai.exe"
"@ | Set-Content -Encoding ASCII (Join-Path $StageDir "start-firebase-emulator.bat")

@"
@echo off
title Evaluasi Metrik AI (Precision@K, NDCG@K, Latensi Rust)
cd /d "%~dp0"
"%~dp0karir-ai.exe" --case-study
echo.
"%~dp0karir-ai.exe" --eval
echo.
pause
"@ | Set-Content -Encoding ASCII (Join-Path $StageDir "cek-evaluasi-ai.bat")

$ZipPath = Join-Path $DistDir "$PkgName.zip"
if (Test-Path $ZipPath) { Remove-Item -Force $ZipPath }
Compress-Archive -Path $StageDir -DestinationPath $ZipPath

$Hash = (Get-FileHash -Algorithm SHA256 $ZipPath).Hash.ToLower()
"$Hash  $PkgName.zip" | Set-Content -Encoding ASCII "$ZipPath.sha256"

Write-Host "[OK] Arsip rilis Windows siap: $ZipPath" -ForegroundColor Green
