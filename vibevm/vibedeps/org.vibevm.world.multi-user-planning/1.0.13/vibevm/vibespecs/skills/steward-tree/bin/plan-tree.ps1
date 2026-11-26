# Windows launcher for plan-tree: runs the Python launcher that sits beside this
# file with the first Python found (the py launcher, then python, then python3).
$launcher = Join-Path $PSScriptRoot 'plan-tree'
if (Get-Command py -ErrorAction SilentlyContinue) {
    & py -3 $launcher @args
} else {
    # -All looks at every PATH entry; the WindowsApps aliases are Store stubs, not Python.
    $python = Get-Command python, python3 -All -ErrorAction SilentlyContinue |
        Where-Object { $_.Source -notlike '*\Microsoft\WindowsApps\*' } | Select-Object -First 1
    if (-not $python) {
        [Console]::Error.WriteLine('plan-tree: Python 3 is not on PATH')
        exit 1
    }
    & $python.Source $launcher @args
}
exit $LASTEXITCODE
