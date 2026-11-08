@echo off
cd /d "%~dp0"
echo ========================================================
echo  Starting CaveGame (Release Mode - Distant Horizons LOD)
echo ========================================================
target\release\cave_game.exe
if %ERRORLEVEL% NEQ 0 (
    echo.
    echo Game exited with error code %ERRORLEVEL%.
    pause
)
