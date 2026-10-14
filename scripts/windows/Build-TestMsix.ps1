[CmdletBinding()]
param([string]$OutputDirectory, [ValidateRange(1, 365)][int]$CertificateValidityDays = 14, [switch]$CiIdentity)
$ErrorActionPreference = 'Stop'
$root = (Resolve-Path "$PSScriptRoot/../..").Path
if (!$OutputDirectory) { $OutputDirectory = Join-Path $root 'src-tauri/target/msix/test' }
[void](New-Item $OutputDirectory -ItemType Directory -Force)
$subject = 'CN=Lenz MSIX Test'
$name = 'app.lenz.desktop'
if ($CiIdentity) {
    if ($env:GITHUB_ACTIONS -ne 'true' -or $env:RUNNER_ENVIRONMENT -ne 'github-hosted') { throw '-CiIdentity is reserved for disposable GitHub-hosted runners.' }
    $subject = "CN=Lenz MSIX CI $env:GITHUB_RUN_ID"
    $name = 'app.lenz.desktop.ci'
}
$certificate = New-SelfSignedCertificate -Type Custom -Subject $subject -KeyUsage DigitalSignature `
    -CertStoreLocation 'Cert:/CurrentUser/My' -NotAfter (Get-Date).AddDays($CertificateValidityDays) `
    -TextExtension @('2.5.29.37={text}1.3.6.1.5.5.7.3.3', '2.5.29.19={text}')
try {
    Export-Certificate -Cert $certificate -FilePath (Join-Path $OutputDirectory 'Lenz-Test.cer') | Out-Null
    & "$PSScriptRoot/Build-Msix.ps1" -CertificateThumbprint $certificate.Thumbprint -Publisher $subject -IdentityName $name -OutputDirectory $OutputDirectory
    @'
TEST SIGNATURE ONLY
This package is not signed by a publicly trusted publisher. The build script never trusts the certificate.
For a disposable Windows test machine, inspect Lenz-Test.cer and explicitly trust it in Local Machine > Trusted People before installation.
Keep this test identity separate from a future production identity. Production certificates must use the configured Publisher.
No private signing key is included in this folder. Check the certificate's NotAfter date before installation.
'@ | Set-Content (Join-Path $OutputDirectory 'TEST-SIGNATURE.txt') -Encoding UTF8
} finally {
    Remove-Item "Cert:/CurrentUser/My/$($certificate.Thumbprint)" -DeleteKey -ErrorAction SilentlyContinue
}
