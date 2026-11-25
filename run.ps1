param(
    [switch]$Emulator,
    [switch]$Reset,
    [switch]$Eval,
    [switch]$CaseStudy,
    [switch]$Release
)

$ErrorActionPreference = "Stop"
Set-Location $PSScriptRoot

$emuProc = $null
try {
    if ($Emulator) {
        if (-not (Get-Command firebase -ErrorAction SilentlyContinue)) {
            Write-Host "[INFO] Firebase CLI belum terpasang. Memanggil setup.ps1..." -ForegroundColor Yellow
            & "$PSScriptRoot\setup.ps1" -EmulatorDeps
        }
        Write-Host "[1/2] Menyalakan Firebase Emulator Suite (Auth :9098, Firestore :8088, UI :4040)..." -ForegroundColor Cyan
        $env:FIRESTORE_EMULATOR_HOST = "127.0.0.1:8088"
        $env:FIREBASE_AUTH_EMULATOR_HOST = "127.0.0.1:9098"
        $env:FIREBASE_PROJECT_ID = "demo-karir-ai"

        $emuProc = Start-Process -FilePath "cmd.exe" -ArgumentList "/c firebase emulators:start --project demo-karir-ai --only auth,firestore" -PassThru -WindowStyle Minimized
        for ($i = 0; $i -lt 30; $i++) {
            Start-Sleep -Seconds 1
            try {
                $null = Invoke-WebRequest -Uri "http://127.0.0.1:8088/" -UseBasicParsing -TimeoutSec 2
                Write-Host "[OK] Cloud Firestore Emulator aktif di http://127.0.0.1:8088 (UI: http://127.0.0.1:4040)" -ForegroundColor Green
                break
            } catch {}
        }
    }

    $appArgs = @()
    if ($Reset) { $appArgs += "--reset" }
    if ($Eval) { $appArgs += "--eval" }
    if ($CaseStudy) { $appArgs += "--case-study" }

    # Jika biner siap pakai tersedia di folder rilis, langsung jalankan tanpa perlu Rust!
    $prebuiltExe = Join-Path $PSScriptRoot "karir-ai.exe"
    if (Test-Path $prebuiltExe) {
        if (-not $Eval -and -not $CaseStudy) {
            Start-Job -ScriptBlock { Start-Sleep -Seconds 2; Start-Process "http://127.0.0.1:3000" } | Out-Null
        }
        & $prebuiltExe @appArgs
        exit $LASTEXITCODE
    }

    if (-not (Get-Command cargo -ErrorAction SilentlyContinue)) {
        Write-Host "[INFO] Cargo/Rust belum ditemukan di PATH. Mencoba cek ~/.cargo/bin..." -ForegroundColor Yellow
        $cargoBin = Join-Path $HOME ".cargo\bin"
        if (Test-Path $cargoBin) {
            $env:PATH = "$cargoBin;$env:PATH"
        } else {
            Write-Host "[ERROR] Rust belum terpasang. Jalankan setup.bat ATAU unduh paket ZIP siap pakai dari GitHub Releases (tanpa perlu install Rust!)." -ForegroundColor Red
            exit 1
        }
    }

    if (-not $Eval -and -not $CaseStudy) {
        Start-Job -ScriptBlock { Start-Sleep -Seconds 3; Start-Process "http://127.0.0.1:3000" } | Out-Null
    }

    if ($Release) {
        cargo run --release -- @appArgs
    } else {
        cargo run -- @appArgs
    }
} finally {
    if ($null -ne $emuProc -and -not $emuProc.HasExited) {
        Write-Host "`nMematikan Firebase Emulator..." -ForegroundColor Yellow
        Stop-Process -Id $emuProc.Id -Force -ErrorAction SilentlyContinue
    }
}
