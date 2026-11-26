@echo off
rem Lets cmd.exe users type plan-tree; the work is done by plan-tree.ps1.
powershell -NoProfile -ExecutionPolicy Bypass -File "%~dp0plan-tree.ps1" %*
exit /b %ERRORLEVEL%
