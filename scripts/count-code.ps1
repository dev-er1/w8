$workspace = Resolve-Path (Join-Path $PSScriptRoot "..")

$total = 0
$fileCount = 0

Get-ChildItem $workspace -Directory -Recurse |
Where-Object { $_.Name -eq "src" } |
ForEach-Object {

    Get-ChildItem $_.FullName -Filter *.rs -File -Recurse | ForEach-Object {

        $fileCount++

        $insideTest = $false
        $waitingModuleBrace = $false
        $braceDepth = 0

        $insideBlockComment = $false

        foreach ($line in Get-Content $_.FullName) {

            $trim = $line.Trim()

            if (-not $insideTest -and $trim -match '^\#\[cfg\(test\)\]') {
                $waitingModuleBrace = $true
                continue
            }

            if ($waitingModuleBrace) {

                $open = ([regex]::Matches($line, '\{')).Count

                if ($open -gt 0) {
                    $insideTest = $true
                    $waitingModuleBrace = $false

                    $braceDepth += $open
                    $braceDepth -= ([regex]::Matches($line, '\}')).Count
                }

                continue
            }

            if ($insideTest) {

                $braceDepth += ([regex]::Matches($line, '\{')).Count
                $braceDepth -= ([regex]::Matches($line, '\}')).Count

                if ($braceDepth -le 0) {
                    $insideTest = $false
                    $braceDepth = 0
                }

                continue
            }

            if ($insideBlockComment) {

                if ($trim -match '\*/') {
                    $insideBlockComment = $false
                }

                continue
            }

            if ($trim.Length -eq 0) {
                continue
            }

            if ($trim.StartsWith("//") -and
                -not $trim.StartsWith("///") -and
                -not $trim.StartsWith("//!")) {
                continue
            }

            if ($trim.StartsWith("/*")) {

                if (-not $trim.Contains("*/")) {
                    $insideBlockComment = $true
                }

                continue
            }

            $total++
        }
    }
}

Write-Host ""
Write-Host "Rust files: $fileCount"
Write-Host "Code lines: $total"