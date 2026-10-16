[CmdletBinding()]
param(
    [ValidateSet('x64', 'arm64')][string]$Architecture = 'x64',
    [string]$BinaryPath,
    [string]$LoaderPath,
    [string]$OutputDirectory,
    [string]$MakeNsisPath
)
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/MsixTools.ps1"
$root = (Resolve-Path "$PSScriptRoot/../..").Path
if (!$MakeNsisPath) {
    $command = Get-Command 'makensis.exe', 'makensis' -ErrorAction SilentlyContinue | Select-Object -First 1
    if ($command) { $MakeNsisPath = $command.Source }
    elseif ($env:OS -eq 'Windows_NT') { $MakeNsisPath = Join-Path ${env:ProgramFiles(x86)} 'NSIS/makensis.exe' }
}
if (!$MakeNsisPath -or !(Test-Path $MakeNsisPath -PathType Leaf)) { throw 'Install NSIS 3 (choco install nsis), or supply -MakeNsisPath.' }
if (!$OutputDirectory) { $OutputDirectory = Join-Path $root 'src-tauri/target/exe' }
[void](New-Item $OutputDirectory -ItemType Directory -Force)
$OutputDirectory = (Resolve-Path $OutputDirectory).Path
$work = Join-Path $OutputDirectory ('.build-' + [guid]::NewGuid().ToString('N'))
$previousRustFlags = $env:RUSTFLAGS
$previousEncodedRustFlags = $env:CARGO_ENCODED_RUSTFLAGS
Push-Location $root
try {
    if (!$BinaryPath) {
        if ($env:OS -ne 'Windows_NT') { throw 'Build the MSVC executable on Windows, or supply an existing Windows -BinaryPath.' }
        $target = if ($Architecture -eq 'x64') { 'x86_64-pc-windows-msvc' } else { 'aarch64-pc-windows-msvc' }
        $env:RUSTFLAGS = ($previousRustFlags + ' -C target-feature=+crt-static').Trim()
        if ($previousEncodedRustFlags) { $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedRustFlags + [char]31 + '-C' + [char]31 + 'target-feature=+crt-static' }
        Invoke-LenzTool 'pnpm' @('tauri', 'build', '--no-bundle', '--target', $target)
        $BinaryPath = Join-Path $root "src-tauri/target/$target/release/lenz.exe"
    }
    $prepare = @('scripts/windows/exe-package.mjs', '--binary', (Resolve-Path $BinaryPath).Path, '--output', $work, '--architecture', $Architecture)
    if ($LoaderPath) { $prepare += @('--loader', (Resolve-Path $LoaderPath).Path) }
    $json = & node @prepare
    if ($LASTEXITCODE -ne 0) { throw 'Preparing the standalone EXE payload failed.' }
    $metadata = $json | ConvertFrom-Json
    $output = Join-Path $OutputDirectory "Lenz_$($metadata.version)_windows_$($metadata.architecture)_setup.exe"
    $prefix = if ($env:OS -eq 'Windows_NT') { '/' } else { '-' }
    $arguments = @("$($prefix)V3", "$($prefix)WX", "$($prefix)INPUTCHARSET", 'UTF8',
        ("$($prefix)DPAYLOAD_DIR=" + $work.Replace('$', '$$')),
        ("$($prefix)DOUTPUT_PATH=" + $output.Replace('$', '$$')),
        ("$($prefix)DVERSION=" + $metadata.productVersion),
        ("$($prefix)DDISPLAY_VERSION=" + $metadata.version),
        ("$($prefix)DARCHITECTURE=" + $metadata.architecture),
        ("$($prefix)DAPP_ICON=" + (Join-Path $root 'src-tauri/icons/icon.ico').Replace('$', '$$')))
    if ($metadata.needsLoader) { $arguments += "$($prefix)DHAS_LOADER" }
    Invoke-LenzTool $MakeNsisPath ($arguments + @("$PSScriptRoot/setup.nsi"))
    if (!(Test-Path $output -PathType Leaf)) { throw 'NSIS did not produce the EXE setup.' }
    Write-Output $output
} finally {
    Pop-Location
    $env:RUSTFLAGS = $previousRustFlags
    $env:CARGO_ENCODED_RUSTFLAGS = $previousEncodedRustFlags
    if (Test-Path $work) { Remove-Item $work -Recurse -Force }
}
