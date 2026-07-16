@echo off
setlocal

echo [1/1] Building Tauri release package...
call npm run tauri:build
if errorlevel 1 (
 echo [error] tauri build failed
 exit /b 1
)

echo.
echo Build complete.
echo Output: target\release\CoterEncrypt.exe
endlocal
