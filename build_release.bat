@echo off
echo Building YOLO Dataset Cleaner (Release)...
echo.

cargo build --release

if %ERRORLEVEL% EQU 0 (
    echo.
    echo Build successful! Copying executable to root folder...
    copy /Y "target\release\clean-cs2vision-dataset.exe" "clean-cs2vision-dataset.exe"
    
    if %ERRORLEVEL% EQU 0 (
        echo.
        echo ✓ Done! Executable copied to: clean-cs2vision-dataset.exe
    ) else (
        echo.
        echo × Error: Failed to copy executable
    )
) else (
    echo.
    echo × Error: Build failed
)

echo.
pause
