@echo off
echo === 启动 Bevy LoginScene ===
echo.
set RUST_LOG=info
set RUST_BACKTRACE=1

"target\debug\mir2_bevy.exe"

echo.
echo === 程序已退出 ===
echo 错误代码: %ERRORLEVEL%
echo.
pause
