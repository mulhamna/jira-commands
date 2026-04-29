$ErrorActionPreference = 'Stop'

$packageName = 'jirac'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64 = 'https://github.com/mulhamna/jira-commands/releases/download/v0.24.4/jirac-windows-x86_64.zip'
$checksum64 = '46e865312256e8c6bb4188c6ae9be6d283aa091f4db3dc037f419a129bcfb3c6'

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
