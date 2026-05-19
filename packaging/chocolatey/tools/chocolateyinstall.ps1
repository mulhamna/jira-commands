$ErrorActionPreference = 'Stop'

$packageName = 'jirac'
$toolsDir = Split-Path -Parent $MyInvocation.MyCommand.Definition
$url64 = 'https://github.com/mulhamna/jira-commands/releases/download/v1.1.0/jirac-windows-x86_64.zip'
$checksum64 = '5f76c888a53f84286a9c1f743e5f49e37fc725006809d3480fcdd9479cc77ebb'

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
