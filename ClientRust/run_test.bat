@echo off
cd /d %~dp0
echo === Starting mir2_bevy ===
target\debug\mir2_bevy.exe
echo.
echo === Program exited ===
pause
