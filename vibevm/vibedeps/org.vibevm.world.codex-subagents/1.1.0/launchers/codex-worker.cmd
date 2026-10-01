@echo off
rem vibe:codex-subagents launcher epoch=1 owner=org.vibevm.world/codex-subagents
powershell.exe -NoLogo -NoProfile -ExecutionPolicy Bypass -File "%~dp0codex-worker.ps1" %*
exit /b %ERRORLEVEL%
