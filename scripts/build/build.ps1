$ErrorActionPreference = 'Stop'

$root = Resolve-Path "$PSScriptRoot/../.."

Write-Host "    Checking that CI passes..."
& "$root/scripts/ci/ci.ps1"
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Host "    Cleaning..."
cargo clean --manifest-path "$root/wdt/Cargo.toml"

Write-Host "    Building..."
cargo build --release --manifest-path "$root/wdt/Cargo.toml"
if ($LASTEXITCODE -ne 0) {
    exit $LASTEXITCODE
}

Write-Host "    Copying binary..."
Copy-Item "$root/wdt/target/release/w8c.exe" "$root/w8c.exe"