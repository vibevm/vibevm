@echo off
setlocal
set "CARGO_EXE="
for /f "delims=" %%C in ('where cargo.exe 2^>nul') do if not defined CARGO_EXE set "CARGO_EXE=%%~fC"
if defined CARGO_EXE goto cargo_found
if defined HOME if exist "%HOME%\.cargo\bin\cargo.exe" set "CARGO_EXE=%HOME%\.cargo\bin\cargo.exe"
if defined CARGO_EXE goto cargo_found
if defined USERPROFILE if exist "%USERPROFILE%\.cargo\bin\cargo.exe" set "CARGO_EXE=%USERPROFILE%\.cargo\bin\cargo.exe"
if defined CARGO_EXE goto cargo_found
>&2 echo lifecycle-rust-stack: cargo.exe was not found in sanitized PATH, HOME\.cargo\bin, or USERPROFILE\.cargo\bin; install Rust or expose cargo on PATH
exit /b 127

:cargo_found
set "SYSTEM_DRIVE=%SystemRoot:~0,3%"
set "VSDEVCMD="
for %%E in (Community BuildTools Professional Enterprise) do if not defined VSDEVCMD if exist "%SYSTEM_DRIVE%Program Files\Microsoft Visual Studio\2022\%%E\Common7\Tools\VsDevCmd.bat" set "VSDEVCMD=%SYSTEM_DRIVE%Program Files\Microsoft Visual Studio\2022\%%E\Common7\Tools\VsDevCmd.bat"
set "VS_EXIT=0"
if defined VSDEVCMD call "%VSDEVCMD%" -no_logo >nul
if defined VSDEVCMD set "VS_EXIT=%ERRORLEVEL%"
if not "%VS_EXIT%"=="0" exit /b %VS_EXIT%
"%CARGO_EXE%" build --quiet
set "CHILD_EXIT=%ERRORLEVEL%"
exit /b %CHILD_EXIT%
