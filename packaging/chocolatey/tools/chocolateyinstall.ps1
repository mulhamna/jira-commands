$ErrorActionPreference = 'Stop'

$packageName = 'jirac'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64 = 'https://github.com/mulhamna/jira-commands/releases/download/v0.23.0/jirac-windows-x86_64.zip'
$checksum64 = '9c3c4067e6f7ef4ba2a3b66ae37a3cbaa892ad9deb2196a5436decfbeb1fcf0b'

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
