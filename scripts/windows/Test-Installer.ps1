# Cross-platform orchestration regression tests. All Windows operations are mocked in a temp folder.
$ErrorActionPreference = 'Stop'
$root = Join-Path ([IO.Path]::GetTempPath()) ('lenz-installer-test-' + [guid]::NewGuid().ToString('N'))
[void](New-Item $root -ItemType Directory)
try {
    Copy-Item "$PSScriptRoot/Install-Lenz.ps1" $root
    Set-Content (Join-Path $root 'Lenz.msix') 'fixture only, never deployed'
    @'
function Add-Trace([string]$Step) { Add-Content $env:LENZ_INSTALL_TEST_TRACE $Step }
function Get-LenzPackageMetadata([string]$Path) {
    return [pscustomobject]@{ Name = 'test.lenz'; Publisher = 'CN=Fixture'; Version = '0.1.2.0'; ApplicationId = 'App' }
}
function Add-AppxPackage([string]$Path) {
    Add-Trace 'install'
    if ($env:LENZ_INSTALL_TEST_MODE -eq 'certificate') { throw 'Deployment failed: 0x800B0109' }
    if ($env:LENZ_INSTALL_TEST_MODE -eq 'deployment') { throw 'Deployment failed: 0x80073CF0' }
}
function Get-AppxPackage([string]$Name) {
    return [pscustomobject]@{ Publisher = 'CN=Fixture'; Version = '0.1.2.0'; PackageFamilyName = 'test.lenz_fixture'; PackageFullName = 'test.lenz_0.1.2.0_x64_fixture' }
}
function Install-LenzWebView2 {
    Add-Trace 'webview'
    if ($env:LENZ_INSTALL_TEST_MODE -eq 'webview') { throw 'WebView2 unavailable' }
}
function Start-LenzPackagedApplication([string]$ApplicationId) { Add-Trace 'launch'; return 123 }
'@ | Set-Content (Join-Path $root 'MsixTools.ps1') -Encoding UTF8
    @'
param([string]$PackageName, [string]$Publisher, [string]$ReportPath)
Add-Trace 'ocr'
if ($env:LENZ_INSTALL_TEST_MODE -eq 'ocr') { throw 'No installed OCR language pack' }
'@ | Set-Content (Join-Path $root 'Test-InstalledOcr.ps1') -Encoding UTF8

    function Test-Scenario([string]$Mode, [string]$Options, [int]$ExitCode, [string]$Trace, [string]$Message = '') {
        $tracePath = Join-Path $root 'trace.txt'
        Remove-Item $tracePath -ErrorAction SilentlyContinue
        $start = [Diagnostics.ProcessStartInfo]::new()
        $start.FileName = (Get-Process -Id $PID).Path
        $start.Arguments = '-NoLogo -NoProfile -NonInteractive -ExecutionPolicy Bypass -File "' + (Join-Path $root 'Install-Lenz.ps1') + '" -PackagePath "' + (Join-Path $root 'Lenz.msix') + '" ' + $Options
        $start.UseShellExecute = $false
        $start.RedirectStandardOutput = $true
        $start.RedirectStandardError = $true
        $start.EnvironmentVariables['LENZ_INSTALL_TEST_MODE'] = $Mode
        $start.EnvironmentVariables['LENZ_INSTALL_TEST_TRACE'] = $tracePath
        $process = [Diagnostics.Process]::Start($start)
        try {
            $stdout = $process.StandardOutput.ReadToEndAsync()
            $stderr = $process.StandardError.ReadToEndAsync()
            if (!$process.WaitForExit(15000)) { $process.Kill(); throw "Scenario $Mode timed out." }
            $output = $stdout.Result + $stderr.Result
            if ($process.ExitCode -ne $ExitCode) { throw "Scenario $Mode exited $($process.ExitCode), expected $ExitCode. $output" }
            $actual = if (Test-Path $tracePath) { (Get-Content $tracePath) -join ',' } else { '' }
            if ($actual -ne $Trace) { throw "Scenario $Mode called '$actual', expected '$Trace'." }
            if ($Message -and $output -notmatch $Message) { throw "Scenario $Mode did not explain '$Message': $output" }
            Write-Output "PASS $Mode [$Options]: $actual (exit $ExitCode)"
        } finally { $process.Dispose() }
    }

    Test-Scenario 'default' '-Launch' 0 'install,webview,launch'
    Test-Scenario 'ocr' '-Launch' 0 'install,webview,launch'
    Test-Scenario 'legacy-skip' '-SkipOcrCheck -Launch' 0 'install,webview,launch'
    Test-Scenario 'explicit-check' '-CheckOcr -Launch' 0 'install,webview,ocr,launch'
    Test-Scenario 'ocr' '-CheckOcr -Launch' 0 'install,webview,ocr,launch' 'Optional local OCR check failed'
    Test-Scenario 'certificate' '-Launch' 10 'install' 'not an OCR error'
    Test-Scenario 'deployment' '-Launch' 1 'install' 'before any OCR check'
    Test-Scenario 'webview' '-Launch' 1 'install,webview' 'WebView2 unavailable'
    Test-Scenario 'invalid-options' '-CheckOcr -SkipOcrCheck' 1 '' 'not both'
} finally { Remove-Item $root -Recurse -Force }
