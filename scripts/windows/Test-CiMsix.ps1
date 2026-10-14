# Runs only inside a disposable CI runner. Never changes trust on a developer/user machine.
$ErrorActionPreference = 'Stop'
if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw 'This certificate trust test is restricted to disposable GitHub-hosted runners.' }
. "$PSScriptRoot/MsixTools.ps1"
$root = (Resolve-Path "$PSScriptRoot/../..").Path
$output = Join-Path $root 'src-tauri/target/msix/test'
$info = Get-Content (Join-Path $output 'package-info.json') -Raw | ConvertFrom-Json
if ($info.name -ne 'app.lenz.desktop.ci') { throw 'Refusing to install/remove a production package in the CI test.' }
$trusted = $null
try {
    $trusted = Import-Certificate -FilePath (Join-Path $output 'Lenz-Test.cer') -CertStoreLocation 'Cert:/LocalMachine/TrustedPeople'
    $packagePath = Join-Path $output (Split-Path $info.packagePath -Leaf)
    & "$PSScriptRoot/Install-Lenz.ps1" -PackagePath $packagePath -OcrReportPath (Join-Path $output 'installed-ocr.json')
} finally {
    try {
        Get-AppxPackage -Name $info.name | Where-Object { $_.Publisher -eq $info.publisher } | Remove-AppxPackage
    } finally {
        if ($trusted) { Remove-Item "Cert:/LocalMachine/TrustedPeople/$($trusted.Thumbprint)" -ErrorAction SilentlyContinue }
    }
}
