@echo off
title KarirAI + Firebase Emulator Suite (:4000)
cd /d "%~dp0"
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0run.ps1" -Emulator %*
if errorlevel 1 (
    echo.
    echo [INFO] Terjadi kendala saat menjalankan emulator. Lihat pesan di atas.
    pause
)
