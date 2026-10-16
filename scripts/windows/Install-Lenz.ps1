[CmdletBinding()]
param([Parameter(Mandatory)][string]$PackagePath, [string]$OcrReportPath, [switch]$CheckOcr, [switch]$SkipOcrCheck, [switch]$Launch)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/MsixTools.ps1"
if ($CheckOcr -and $SkipOcrCheck) { throw 'Use either -CheckOcr or -SkipOcrCheck, not both.' }
if ($OcrReportPath -and !$CheckOcr) { throw '-OcrReportPath requires -CheckOcr.' }
$PackagePath = (Resolve-Path $PackagePath).Path
$metadata = Get-LenzPackageMetadata $PackagePath
# Windows deployment validates package hashes, signature, Publisher and machine certificate trust.
# Use its MSIX policy (including TrustedPeople), rather than a generic Authenticode status filter.
# No -AllowUnsigned/-Register and no certificate import: trust remains an explicit separate step.
try { Add-AppxPackage -Path $PackagePath }
catch {
    if ($_.Exception.Message -match '0x800B0109|0x800B010A') {
        [Console]::Error.WriteLine("MSIX certificate trust failed (not an OCR error): $($_.Exception.Message). Inspect the public test certificate, then explicitly install it in Local Machine > Trusted People before retrying. Current User is not sufficient. This installer does not change certificate trust.")
        exit 10
    }
    throw "MSIX deployment failed (before any OCR check): $($_.Exception.Message)"
}
$package = Get-AppxPackage -Name $metadata.Name | Where-Object { $_.Publisher -eq $metadata.Publisher } | Select-Object -First 1
if (!$package -or [string]$package.Version -ne $metadata.Version) { throw 'The expected MSIX version was not installed.' }
Install-LenzWebView2
# Local OCR and its language packs are optional. The legacy -SkipOcrCheck remains accepted.
if ($CheckOcr) {
    try { & "$PSScriptRoot/Test-InstalledOcr.ps1" -PackageName $metadata.Name -Publisher $metadata.Publisher -ReportPath $OcrReportPath }
    catch { Write-Warning "Optional local OCR check failed: $($_.Exception.Message). Lenz remains installed. Configure Tencent/another OCR service, or install Windows OCR language packs later." }
}
if ($Launch) { [void](Start-LenzPackagedApplication "$($package.PackageFamilyName)!$($metadata.ApplicationId)") }
Write-Output "Installed $($package.PackageFullName). Open Lenz from the Start menu."
