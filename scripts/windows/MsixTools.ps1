Set-StrictMode -Version Latest

function Get-LenzSdkTool([string]$Name) {
    $kits = Join-Path ${env:ProgramFiles(x86)} 'Windows Kits/10/bin'
    $tools = @(Get-ChildItem $kits -Directory | Where-Object { $_.Name -match '^10\.' } |
        Sort-Object { [version]$_.Name } -Descending |
        ForEach-Object { Join-Path $_.FullName "x64/$Name" } | Where-Object { Test-Path $_ })
    if (!$tools.Count) { throw "Install the Windows 10/11 SDK ($Name is missing)." }
    return $tools[0]
}

function Invoke-LenzTool([string]$Program, [string[]]$Arguments) {
    & $Program @Arguments
    if ($LASTEXITCODE -ne 0) { throw "$Program failed with exit code $LASTEXITCODE." }
}

function Get-LenzPackageMetadata([string]$Path) {
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    $archive = [IO.Compression.ZipFile]::OpenRead((Resolve-Path $Path).Path)
    try {
        $entry = $archive.GetEntry('AppxManifest.xml')
        if (!$entry) { throw 'Missing AppxManifest.xml.' }
        $reader = [IO.StreamReader]::new($entry.Open())
        try { [xml]$manifest = $reader.ReadToEnd() } finally { $reader.Dispose() }
        return [pscustomobject]@{
            Name = [string]$manifest.Package.Identity.Name
            Publisher = [string]$manifest.Package.Identity.Publisher
            Version = [string]$manifest.Package.Identity.Version
            Architecture = [string]$manifest.Package.Identity.ProcessorArchitecture
            ApplicationId = [string]$manifest.Package.Applications.Application.Id
        }
    } finally { $archive.Dispose() }
}

function Start-LenzPackagedApplication([string]$ApplicationId, [string]$Arguments = '') {
    if (!('LenzPackageActivation' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
[ComImport, Guid("2e941141-7f97-4756-ba1d-9decde894a3d"), InterfaceType(ComInterfaceType.InterfaceIsIUnknown)]
interface ILenzActivationManager {
    [PreserveSig]
    int ActivateApplication([MarshalAs(UnmanagedType.LPWStr)] string id, [MarshalAs(UnmanagedType.LPWStr)] string args, uint options, out uint processId);
    [PreserveSig]
    int ActivateForFile([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr itemArray, [MarshalAs(UnmanagedType.LPWStr)] string verb, out uint processId);
    [PreserveSig]
    int ActivateForProtocol([MarshalAs(UnmanagedType.LPWStr)] string id, IntPtr itemArray, out uint processId);
}
public static class LenzPackageActivation {
    public static uint Start(string id, string args) {
        var manager = (ILenzActivationManager)Activator.CreateInstance(Type.GetTypeFromCLSID(new Guid("45ba127d-10a8-46ea-8ab7-56ea9078943c")));
        try { uint pid; Marshal.ThrowExceptionForHR(manager.ActivateApplication(id, args, 0, out pid)); return pid; }
        finally { Marshal.ReleaseComObject(manager); }
    }
}
'@
    }
    return [LenzPackageActivation]::Start($ApplicationId, $Arguments)
}

function Test-LenzWebView2 {
    $client = '{F3017226-FE2A-4295-8BDF-00C3A9A7E4C5}'
    foreach ($key in @("HKCU:/Software/Microsoft/EdgeUpdate/Clients/$client", "HKLM:/SOFTWARE/WOW6432Node/Microsoft/EdgeUpdate/Clients/$client", "HKLM:/SOFTWARE/Microsoft/EdgeUpdate/Clients/$client")) {
        $value = Get-ItemProperty $key -Name pv -ErrorAction SilentlyContinue
        if ($value -and $value.pv -and [version]$value.pv -gt [version]'0.0.0.0') { return $true }
    }
    return $false
}

function Install-LenzWebView2 {
    if (Test-LenzWebView2) { return }
    $installer = Join-Path ([IO.Path]::GetTempPath()) ("lenz-webview2-" + [guid]::NewGuid().ToString('N') + '.exe')
    try {
        Invoke-WebRequest 'https://go.microsoft.com/fwlink/p/?LinkId=2124703' -OutFile $installer -UseBasicParsing
        $signature = Get-AuthenticodeSignature $installer
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'O=Microsoft Corporation') {
            throw 'The WebView2 installer must have a valid Microsoft signature.'
        }
        $process = Start-Process $installer -ArgumentList '/silent', '/install' -PassThru -Wait
        if ($process.ExitCode -ne 0 -or !(Test-LenzWebView2)) { throw 'WebView2 installation failed.' }
    } finally { Remove-Item $installer -ErrorAction SilentlyContinue }
}
