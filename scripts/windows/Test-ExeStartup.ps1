[CmdletBinding()]
param(
    [Parameter(Mandatory = $true)][string]$BinaryPath,
    [string[]]$LaunchArguments = @(),
    [switch]$ExpectMainWindow,
    [string]$ReportPath = (Join-Path ([IO.Path]::GetTempPath()) 'lenz-exe-startup.json')
)
$ErrorActionPreference = 'Stop'
Set-StrictMode -Version Latest
if ($env:OS -ne 'Windows_NT') { throw 'Run this startup check on Windows.' }
if (!('LenzStartupWindows' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Text;
using System.Runtime.InteropServices;
public static class LenzStartupWindows {
    delegate bool Callback(IntPtr window, IntPtr data);
    [DllImport("user32.dll")] static extern bool EnumWindows(Callback callback, IntPtr data);
    [DllImport("user32.dll")] static extern bool IsWindowVisible(IntPtr window);
    [DllImport("user32.dll")] static extern uint GetWindowThreadProcessId(IntPtr window, out uint processId);
    [DllImport("user32.dll", CharSet = CharSet.Unicode)] static extern int GetWindowText(IntPtr window, StringBuilder text, int count);
    public static bool HasVisibleMainWindow(int processId) {
        bool found = false;
        EnumWindows((window, data) => {
            uint owner; GetWindowThreadProcessId(window, out owner);
            if (owner != processId || !IsWindowVisible(window)) return true;
            var title = new StringBuilder(256); GetWindowText(window, title, title.Capacity);
            found = title.ToString() == "Lenz";
            return !found;
        }, IntPtr.Zero);
        return found;
    }
}
'@
}
$binary = (Resolve-Path $BinaryPath).Path
$work = Join-Path ([IO.Path]::GetTempPath()) ('lenz-startup-' + [guid]::NewGuid().ToString('N'))
[void](New-Item $work -ItemType Directory)
$process = $null
$previousNoDialog = $env:LENZ_STARTUP_NO_DIALOG
$report = [ordered]@{ processAlive = $false; setupComplete = $false; mainWindowShown = $false; exitCode = $null; stdout = ''; stderr = '' }
try {
    $started = Get-Date
    $env:LENZ_STARTUP_NO_DIALOG = '1'
    $startOptions = @{
        FilePath = $binary; WorkingDirectory = (Split-Path $binary); PassThru = $true
        RedirectStandardOutput = (Join-Path $work 'stdout.txt'); RedirectStandardError = (Join-Path $work 'stderr.txt')
    }
    if ($LaunchArguments.Count) { $startOptions.ArgumentList = $LaunchArguments }
    $process = Start-Process @startOptions
    $deadline = $started.AddSeconds(30)
    do {
        Start-Sleep -Milliseconds 500
        $process.Refresh()
        $report.processAlive = !$process.HasExited
        if (!$report.processAlive) { $process.WaitForExit(); $report.exitCode = $process.ExitCode; break }
        $report.mainWindowShown = [LenzStartupWindows]::HasVisibleMainWindow($process.Id)
        $report.stdout = [IO.File]::ReadAllText((Join-Path $work 'stdout.txt'))
        $report.setupComplete = [bool]($report.stdout -match 'Lenz setup complete')
        if ($report.setupComplete -and (!$ExpectMainWindow -or $report.mainWindowShown)) { break }
    } while ((Get-Date) -lt $deadline)
    # Let the frontend finish its first IPC requests; a surviving empty process is insufficient.
    if ($report.processAlive) { Start-Sleep -Seconds 2; $process.Refresh(); $report.processAlive = !$process.HasExited }
    if (!$report.processAlive -or !$report.setupComplete -or ($ExpectMainWindow -and !$report.mainWindowShown)) {
        throw 'The installed EXE did not finish startup. See the startup report.'
    }
} finally {
    if ($process -and !$process.HasExited) { Stop-Process -Id $process.Id -Force; $process.WaitForExit() }
    $env:LENZ_STARTUP_NO_DIALOG = $previousNoDialog
    foreach ($stream in @('stdout', 'stderr')) {
        $path = Join-Path $work "$stream.txt"
        if (Test-Path $path) { $report[$stream] = [IO.File]::ReadAllText($path) }
    }
    $json = $report | ConvertTo-Json -Depth 5
    Set-Content $ReportPath $json -Encoding UTF8
    Write-Output $json
    Remove-Item $work -Recurse -Force
}
