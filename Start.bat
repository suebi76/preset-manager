@echo off
rem ---------------------------------------------------------------------------
rem Preset Manager starten
rem
rem   Doppelklick                  -> Fenster mit den installierten Presets
rem   Ordner/ZIP auf diese Datei   -> Fenster oeffnet sich mit der Importvorschau
rem   ziehen
rem
rem Beim ersten Start wird das Programm uebersetzt; das dauert einige Minuten.
rem Danach startet es sofort.
rem ---------------------------------------------------------------------------
setlocal
cd /d "%~dp0"
title Preset Manager

where cargo >nul 2>nul
if errorlevel 1 (
    echo Rust ist nicht installiert oder nicht im PATH.
    echo Installation: https://rustup.rs
    pause
    exit /b 1
)

echo Programm wird vorbereitet ...
cargo build --release --quiet -p preset_manager --bin preset_manager
if errorlevel 1 (
    echo.
    echo Das Programm konnte nicht uebersetzt werden.
    pause
    exit /b 1
)

start "" "target\release\preset_manager.exe" %*
endlocal
