[CmdletBinding()]
param([string]$PackageName = 'app.lenz.desktop', [string]$Publisher, [string]$ReportPath, [ValidateRange(1, 300)][int]$TimeoutSeconds = 45)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/MsixTools.ps1"
$packages = @(Get-AppxPackage -Name $PackageName)
if ($Publisher) { $packages = @($packages | Where-Object { $_.Publisher -eq $Publisher }) }
if ($packages.Count -ne 1) { throw 'Exactly one installed Lenz package is required.' }
$package = $packages[0]
[xml]$manifest = Get-Content (Join-Path $package.InstallLocation 'AppxManifest.xml') -Raw
$aumid = "$($package.PackageFamilyName)!$($manifest.Package.Applications.Application.Id)"
if (!$ReportPath) { $ReportPath = Join-Path ([IO.Path]::GetTempPath()) ('lenz-ocr-' + [guid]::NewGuid().ToString('N') + '.json') }
$ReportPath = [IO.Path]::GetFullPath($ReportPath)
if (Test-Path $ReportPath) { throw 'Use a new diagnostic report path.' }
# ActivateApplication is necessary: executing a copy of the exe does not prove package identity.
$activationTime = [DateTime]::UtcNow
$processId = Start-LenzPackagedApplication $aumid ('--diagnose-windows-ocr "' + $ReportPath + '"')
$deadline = [DateTime]::UtcNow.AddSeconds($TimeoutSeconds)
while (!(Test-Path $ReportPath) -and [DateTime]::UtcNow -lt $deadline) { Start-Sleep -Milliseconds 250 }
if (!(Test-Path $ReportPath)) {
    $process = Get-Process -Id $processId -ErrorAction SilentlyContinue
    # Never terminate a previously running app if Windows reused its activation.
    if ($process -and $process.StartTime.ToUniversalTime() -ge $activationTime) { $process | Stop-Process }
    throw "The installed OCR diagnostic timed out. See $ReportPath."
}
$report = Get-Content $ReportPath -Raw | ConvertFrom-Json
if ($report.packageIdentity -ne $package.PackageFullName -or !$report.ok -or !$report.blocks.Count -or $report.recognizedText -cnotmatch 'LENZ' -or $report.recognizedText -notmatch '12345') {
    throw "Installed OCR validation failed: $($report.error). Report: $ReportPath"
}
Write-Output "Installed Windows OCR passed. Package: $($report.packageIdentity). Report: $ReportPath"
