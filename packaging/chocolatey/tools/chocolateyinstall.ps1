$ErrorActionPreference = 'Stop'

$packageName = 'jirac'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64 = 'https://github.com/mulhamna/jira-commands/releases/download/v0.29.0/jirac-windows-x86_64.zip'
$checksum64 = 'edadbdac8a6b84e4050ff3152769f6171ef9b85e691065068af5fd8bc50ffa7a'

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
