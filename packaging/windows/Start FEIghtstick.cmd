@echo off

powershell.exe -NoProfile -NonInteractive -Command "Start-ScheduledTask -TaskName 'FEIghtstick'"
if errorlevel 1 pause
