[CmdletBinding()]
param(
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
    [string]$CertificateThumbprint,
    [string]$Publisher,
    [string]$IdentityName,
    [string]$PackageVersion,
    [string]$BinaryPath,
    [string]$LoaderPath,
    [string]$TimestampUrl,
    [string]$OutputDirectory,
    [switch]$Unsigned
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/MsixTools.ps1"
$root = (Resolve-Path "$PSScriptRoot/../..").Path
$target = if ($Architecture -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'aarch64-pc-windows-msvc' }
if (!$OutputDirectory) { $OutputDirectory = Join-Path $root "src-tauri/target/msix/$Architecture" }
[void](New-Item $OutputDirectory -ItemType Directory -Force)
$OutputDirectory = (Resolve-Path $OutputDirectory).Path
$work = Join-Path $OutputDirectory ('.build-' + [guid]::NewGuid().ToString('N'))
$signer = $null
if (!$Unsigned) {
    if (!$CertificateThumbprint) { throw 'Supply -CertificateThumbprint, or use -Unsigned for package inspection only.' }
    $signer = Get-Item "Cert:/CurrentUser/My/$CertificateThumbprint"
    if (!$signer.HasPrivateKey) { throw 'The code-signing certificate has no private key.' }
    if (!$Publisher) { $Publisher = $signer.Subject }
    if ($Publisher -ne $signer.Subject) { throw 'Publisher must exactly match the signing certificate Subject.' }
}
$previousRustFlags = $env:RUSTFLAGS
$previousEncodedRustFlags = $env:CARGO_ENCODED_RUSTFLAGS
Push-Location $root
try {
    if (!$BinaryPath) {
        # A self-contained MSVC executable avoids a missing VC++ runtime after clean installation.
        $env:RUSTFLAGS = ($previousRustFlags + ' -C target-feature=+crt-static').Trim()
        if ($previousEncodedRustFlags) { $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedRustFlags + [char]31 + '-C' + [char]31 + 'target-feature=+crt-static' }
        Invoke-LenzTool 'pnpm' @('tauri', 'build', '--no-bundle', '--target', $target, '--config', 'src-tauri/tauri.windows.conf.json')
        $BinaryPath = Join-Path $root "src-tauri/target/$target/release/lenz.exe"
    }
    $packageArgs = @('scripts/windows/msix-package.mjs', '--binary', (Resolve-Path $BinaryPath).Path, '--output', $work, '--architecture', $Architecture)
    if ($Publisher) { $packageArgs += @('--publisher', $Publisher) }
    if ($IdentityName) { $packageArgs += @('--name', $IdentityName) }
    if ($PackageVersion) { $packageArgs += @('--version', $PackageVersion) }
    if ($LoaderPath) { $packageArgs += @('--loader', (Resolve-Path $LoaderPath).Path) }
    $output = & node @packageArgs
    if ($LASTEXITCODE -ne 0) { throw 'Preparing the MSIX package failed.' }
    $metadata = $output | ConvertFrom-Json
    $suffix = if ($Unsigned) { '-unsigned' } else { '' }
    $path = Join-Path $OutputDirectory ("Lenz-$($metadata.version)-$Architecture$suffix.msix")
    Invoke-LenzTool (Get-LenzSdkTool 'makeappx.exe') @('pack', '/d', $work, '/p', $path, '/o', '/h', 'SHA256')
    if (!$Unsigned) {
        $signArgs = @('sign', '/fd', 'SHA256', '/sha1', $signer.Thumbprint, '/s', 'My')
        if ($TimestampUrl) { $signArgs += @('/tr', $TimestampUrl, '/td', 'SHA256') }
        Invoke-LenzTool (Get-LenzSdkTool 'signtool.exe') ($signArgs + @($path))
    }
    $metadata | Add-Member -NotePropertyName packagePath -NotePropertyValue $path
    $metadata | ConvertTo-Json | Set-Content (Join-Path $OutputDirectory 'package-info.json') -Encoding UTF8
    Copy-Item "$PSScriptRoot/Install-Lenz.ps1", "$PSScriptRoot/MsixTools.ps1", "$PSScriptRoot/Test-InstalledOcr.ps1" $OutputDirectory -Force
    Write-Output $path
} finally {
    Pop-Location
    $env:RUSTFLAGS = $previousRustFlags
    $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedRustFlags
    if (Test-Path $work) { Remove-Item $work -Recurse -Force }
}
