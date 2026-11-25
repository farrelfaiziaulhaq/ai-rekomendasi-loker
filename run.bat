@echo off
title KarirAI - Sistem Rekomendasi Lowongan Kerja (Rust + Firebase)
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run.ps1" %*
if errorlevel 1 (
    echo.
    echo [INFO] Terjadi kendala saat menjalankan aplikasi. Lihat pesan di atas.
    pause
)
