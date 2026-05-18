$ErrorActionPreference = "Stop"

$repoRoot = Resolve-Path (Join-Path $PSScriptRoot "..")
$extensionDir = Join-Path $repoRoot "browser-extension"
$outputDir = Join-Path $repoRoot "dist"
$zipPath = Join-Path $outputDir "coter-cookie-bridge.zip"

if (-not (Test-Path $extensionDir)) {
 throw "Extension directory not found: $extensionDir"
}

New-Item -ItemType Directory -Force -Path $outputDir | Out-Null

if (Test-Path $zipPath) {
 Remove-Item -LiteralPath $zipPath -Force
}

Compress-Archive -Path (Join-Path $extensionDir "*") -DestinationPath $zipPath -Force

Write-Host "Browser extension package created:"
Write-Host $zipPath
