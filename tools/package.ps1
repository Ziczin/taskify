param(
    [string]$OutputRoot = "",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"

$root = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
if ([string]::IsNullOrWhiteSpace($OutputRoot)) {
    $OutputRoot = Join-Path $root "dist"
}
$OutputRoot = [System.IO.Path]::GetFullPath($OutputRoot)

if (-not $SkipBuild) {
    & cargo build --release --locked
    if ($LASTEXITCODE -ne 0) {
        throw "cargo build failed with exit code $LASTEXITCODE"
    }
}

$exe = Join-Path $root "target\release\taskify.exe"
if (-not (Test-Path -LiteralPath $exe)) {
    throw "not found: $exe"
}

$cargo = Get-Content -LiteralPath (Join-Path $root "Cargo.toml") -Raw
if ($cargo -notmatch '(?m)^version\s*=\s*"([^"]+)"') {
    throw "cannot read version from Cargo.toml"
}
$semver = $Matches[1]
$version = ($semver -split '\.')[0]
$name = "taskify-$version-windows-x64"
$PackagePrefix = "taskify-*-windows-x64"

if (Test-Path -LiteralPath $OutputRoot) {
    Get-ChildItem -LiteralPath $OutputRoot |
        Where-Object { $_.Name -like $PackagePrefix -or $_.Name -like "$PackagePrefix.zip" } |
        Remove-Item -Recurse -Force
}

$packageDir = Join-Path $OutputRoot $name

if (Test-Path -LiteralPath $packageDir) {
    Remove-Item -LiteralPath $packageDir -Recurse -Force
}
New-Item -ItemType Directory -Path $packageDir -Force | Out-Null

Copy-Item -LiteralPath $exe -Destination (Join-Path $packageDir "taskify.exe")
Copy-Item -LiteralPath (Join-Path $root "assets") -Destination $packageDir -Recurse
Copy-Item -LiteralPath (Join-Path $root "README.md") -Destination $packageDir
Copy-Item -LiteralPath (Join-Path $root "README.ru.md") -Destination (Join-Path $packageDir "README.ru.md")
Copy-Item -LiteralPath (Join-Path $root "LICENSE") -Destination $packageDir

$zip = Join-Path $OutputRoot "$name.zip"
if (Test-Path -LiteralPath $zip) {
    Remove-Item -LiteralPath $zip -Force
}
Compress-Archive -LiteralPath $packageDir -DestinationPath $zip -CompressionLevel Optimal

Write-Output $zip
