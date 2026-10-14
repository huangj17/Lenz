[CmdletBinding()]
param([Parameter(Mandatory)][string]$PackagePath, [string]$OcrReportPath, [switch]$SkipOcrCheck, [switch]$Launch)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/MsixTools.ps1"
$PackagePath = (Resolve-Path $PackagePath).Path
$metadata = Get-LenzPackageMetadata $PackagePath
# Windows deployment validates package hashes, signature, Publisher and machine certificate trust.
# Use its MSIX policy (including TrustedPeople), rather than a generic Authenticode status filter.
# No -AllowUnsigned/-Register and no certificate import: trust remains an explicit separate step.
try { Add-AppxPackage -Path $PackagePath }
catch { throw "MSIX deployment failed: $($_.Exception.Message). Test certificates require a separate, explicit trust step; this installer does not change certificate trust." }
$package = Get-AppxPackage -Name $metadata.Name | Where-Object { $_.Publisher -eq $metadata.Publisher } | Select-Object -First 1
if (!$package -or [string]$package.Version -ne $metadata.Version) { throw 'The expected MSIX version was not installed.' }
Install-LenzWebView2
if (!$SkipOcrCheck) { & "$PSScriptRoot/Test-InstalledOcr.ps1" -PackageName $metadata.Name -Publisher $metadata.Publisher -ReportPath $OcrReportPath }
if ($Launch) { [void](Start-LenzPackagedApplication "$($package.PackageFamilyName)!$($metadata.ApplicationId)") }
Write-Output "Installed $($package.PackageFullName). Open Lenz from the Start menu."
