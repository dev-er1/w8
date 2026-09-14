$ErrorActionPreference = 'Stop'

$root = Resolve-Path "$PSScriptRoot/../.."

$workspaces = @('w8', 'wdt') | ForEach-Object { Join-Path $root $_ }

$steps = @(
    @{ Name = 'Check';  Command = { param($ws) cargo check --manifest-path "$ws/Cargo.toml" } }
    @{ Name = 'Format'; Command = { param($ws) cargo fmt --all --check --manifest-path "$ws/Cargo.toml" } }
    @{ Name = 'Clippy'; Command = { param($ws) cargo clippy --all-targets --all-features --manifest-path "$ws/Cargo.toml" -- -D warnings } }
    @{ Name = 'Build';  Command = { param($ws) cargo build --release --manifest-path "$ws/Cargo.toml" } }
    @{ Name = 'Test';   Command = { param($ws) cargo test --workspace --release --manifest-path "$ws/Cargo.toml" } }
)

foreach ($ws in $workspaces) {
    foreach ($step in $steps) {
        Write-Host "======> [$(Split-Path $ws -Leaf)] $($step.Name)"

        & $step.Command $ws

        if ($LASTEXITCODE -ne 0) {
            Write-Error "[$(Split-Path $ws -Leaf)] $($step.Name) failed with exit code $LASTEXITCODE"
            exit $LASTEXITCODE
        }
    }
}

Write-Host "`n"
Write-Host "        CI passed!"
