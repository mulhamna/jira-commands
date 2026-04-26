$ErrorActionPreference = 'Stop'

$packageName = 'jirac'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64 = 'https://github.com/mulhamna/jira-commands/releases/download/v0.21.0/jirac-windows-x86_64.zip'
$checksum64 = 'e82c29a2b92be0c4bc40ddf3847dd98fd8788e99d48a12368cc9df9d39e3f058'

$packageArgs = @{
  packageName    = $packageName
  unzipLocation  = $toolsDir
  fileType       = 'zip'
  url64bit       = $url64
  checksum64     = $checksum64
  checksumType64 = 'sha256'
}

Install-ChocolateyZipPackage @packageArgs

$exePath = Get-ChildItem -Path $toolsDir -Filter 'jirac*.exe' | Select-Object -First 1
if (-not $exePath) {
  throw 'Expected jirac.exe not found after install'
}

Install-BinFile -Name 'jirac' -Path $exePath.FullName
