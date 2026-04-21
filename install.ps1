param(
    [string]$Binary = "jirac",
    [string]$InstallDir = "$env:LOCALAPPDATA\Programs\jirac\bin",
    [switch]$SkipPathUpdate
)

$ErrorActionPreference = "Stop"

$Repo = "mulhamna/jira-commands"
$SupportedBinaries = @("jirac", "jirac-mcp")

if ($SupportedBinaries -notcontains $Binary) {
    throw "Unsupported -Binary '$Binary'. Use 'jirac' or 'jirac-mcp'."
}

function Write-Info($msg) {
    Write-Host "==> $msg" -ForegroundColor Green
}

function Get-LatestReleaseTag {
    $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
    if (-not $release.tag_name) {
        throw "Could not determine latest release tag."
    }
    return $release.tag_name
}

function Get-AssetName($BinaryName) {
    switch ($BinaryName) {
        "jirac" { return "jirac-windows-x86_64.zip" }
        "jirac-mcp" { return "jirac-mcp-windows-x86_64.zip" }
        default { throw "Unsupported binary '$BinaryName'." }
    }
}

function Ensure-InstallDir($Path) {
    if (-not (Test-Path -LiteralPath $Path)) {
        New-Item -ItemType Directory -Force -Path $Path | Out-Null
    }
}

function Add-ToUserPath($Path) {
    $current = [Environment]::GetEnvironmentVariable("Path", "User")
    $parts = @()
    if ($current) {
        $parts = $current.Split(';', [System.StringSplitOptions]::RemoveEmptyEntries)
    }

    if ($parts -contains $Path) {
        return $false
    }

    $newPath = if ($current) { "$current;$Path" } else { $Path }
    [Environment]::SetEnvironmentVariable("Path", $newPath, "User")
    return $true
}

Write-Host ""
Write-Host "$Binary installer" -ForegroundColor Cyan
Write-Host "  Jira tooling for Windows terminals and MCP clients"
Write-Host ""

$tag = Get-LatestReleaseTag
$assetName = Get-AssetName $Binary
$baseUrl = "https://github.com/$Repo/releases/download/$tag"
$assetUrl = "$baseUrl/$assetName"

Write-Info "Latest release: $tag"
Write-Info "Downloading $assetName"

$tempDir = Join-Path ([System.IO.Path]::GetTempPath()) ("jirac-install-" + [System.Guid]::NewGuid().ToString("N"))
$zipPath = Join-Path $tempDir $assetName
$extractDir = Join-Path $tempDir "extract"

New-Item -ItemType Directory -Force -Path $tempDir | Out-Null
try {
    Invoke-WebRequest -Uri $assetUrl -OutFile $zipPath
    Expand-Archive -LiteralPath $zipPath -DestinationPath $extractDir -Force

    Ensure-InstallDir $InstallDir

    $exeName = if ($Binary -eq "jirac") { "jirac.exe" } else { "jirac-mcp.exe" }
    $sourceExe = Get-ChildItem -Path $extractDir -Recurse -Filter $exeName | Select-Object -First 1
    if (-not $sourceExe) {
        throw "Could not find $exeName inside downloaded archive."
    }

    $targetExe = Join-Path $InstallDir $exeName
    Copy-Item -LiteralPath $sourceExe.FullName -Destination $targetExe -Force

    Write-Host ""
    Write-Host "$Binary $tag installed to $targetExe" -ForegroundColor Green

    if ($SkipPathUpdate) {
        Write-Host "Skipped PATH update because -SkipPathUpdate was set." -ForegroundColor DarkGray
    } elseif (Add-ToUserPath $InstallDir) {
        Write-Host "Added $InstallDir to your user PATH. Restart PowerShell to use it." -ForegroundColor Yellow
    } else {
        Write-Host "$InstallDir is already present in your user PATH." -ForegroundColor DarkGray
    }

    if ($Binary -eq "jirac") {
        Write-Host "Run: jirac auth login"
    } else {
        Write-Host "Run: jirac-mcp serve --transport stdio"
    }

    Write-Host "Docs: https://github.com/$Repo"
}
finally {
    if (Test-Path -LiteralPath $tempDir) {
        Remove-Item -LiteralPath $tempDir -Recurse -Force
    }
}
