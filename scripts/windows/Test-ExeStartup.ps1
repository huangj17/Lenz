[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$BinaryPath,
    [switch]$ExpectMainWindow,
    [string]$ReportPath = (Join-Path ([IO.Path]::GetTempPath()) 'lenz-exe-startup.json')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT') { throw 'Run this startup check on Windows.' }
$binary = (Resolve-Path $BinaryPath).Path
$work = Join-Path ([IO.Path]::GetTempPath()) ('lenz-startup-' + [guid]::NewGuid().ToString('N'))
[void](New-Item $work -ItemType Directory)
$process = $null
$report = [ordered]@{ processAlive = $false; setupComplete = $false; mainWindowShown = $false; exitCode = $null; stdout = ''; stderr = '' }
try {
    $started = Get-Date
    $process = Start-Process $binary -WorkingDirectory (Split-Path $binary) -PassThru `
        -RedirectStandardOutput (Join-Path $work 'stdout.txt') -RedirectStandardError (Join-Path $work 'stderr.txt')
    $deadline = $started.AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        $report.processAlive = !$process.HasExited
        if (!$report.processAlive) { $process.WaitForExit(); $report.exitCode = $process.ExitCode; break }
        $report.mainWindowShown = $process.MainWindowHandle -ne [IntPtr]::Zero
        $report.stdout = Get-Content (Join-Path $work 'stdout.txt') -Raw -ErrorAction SilentlyContinue
        $report.setupComplete = [bool]($report.stdout -match 'Lenz setup complete')
        if ($report.setupComplete -and (!$ExpectMainWindow -or $report.mainWindowShown)) { break }
    } while ((Get-Date) -lt $deadline)
    if (!$report.processAlive -or !$report.setupComplete -or ($ExpectMainWindow -and !$report.mainWindowShown)) {
        throw 'The installed EXE did not finish startup. See the startup report.'
    }
} finally {
    if ($process -and !$process.HasExited) { Stop-Process -Id $process.Id -Force; $process.WaitForExit() }
    $report.stdout = Get-Content (Join-Path $work 'stdout.txt') -Raw -ErrorAction SilentlyContinue
    $report.stderr = Get-Content (Join-Path $work 'stderr.txt') -Raw -ErrorAction SilentlyContinue
    $json = $report | ConvertTo-Json -Depth 5
    Set-Content $ReportPath $json -Encoding UTF8
    Write-Output $json
    Remove-Item $work -Recurse -Force
}
