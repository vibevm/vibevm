@echo off
rem vibe:zai-glm-claude launcher epoch=1 owner=org.vibevm.world/zai-glm-claude
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0claudez.ps1" %*
exit /b %ERRORLEVEL%
