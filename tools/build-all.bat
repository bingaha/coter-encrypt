@echo off
chcp 65001 >nul
echo ========================================
echo 加解密工具 - 单体 JAR 打包脚本
echo ========================================
echo.

REM 切换到项目根目录（脚本所在目录的上一级）
cd /d "%~dp0.."

echo [1/3] 构建前端...
cd frontend
call npm install
if %errorlevel% neq 0 (
 echo [错误] npm install 失败
 pause
 exit /b 1
)
call npm run build
if %errorlevel% neq 0 (
 echo [错误] npm run build 失败
 pause
 exit /b 1
)
echo [1/3] 前端构建完成
echo.

echo [2/3] 复制前端产物到后端资源目录...
cd /d "%~dp0.."
if exist "\src\main\resources\static" rmdir /s /q "\src\main\resources\static"
xcopy "frontend\dist" "\src\main\resources\static\" /E /I /Q
echo [2/3] 复制完成
echo.

echo [3/3] 打包后端 JAR...
cd 
call mvn clean package -DskipTests
if %errorlevel% neq 0 (
 echo [错误] 打包失败
 pause
 exit /b 1
)
echo [3/3] 打包完成
echo.

echo ========================================
echo 打包成功！JAR 文件位于:
echo \target\-1.0-SNAPSHOT.jar
echo.
echo 启动命令:
echo -jar \target\-1.0-SNAPSHOT.jar
echo.
echo 然后浏览器访问: http://localhost:8080
echo ========================================
pause
