# Standalone EXE prerequisite only. Defining the shared helpers does not deploy any MSIX.
$ErrorActionPreference = 'Stop'
. "$PSScriptRoot/MsixTools.ps1"
Install-LenzWebView2
