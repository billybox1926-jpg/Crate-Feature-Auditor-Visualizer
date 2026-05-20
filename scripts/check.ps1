$ErrorActionPreference = "Stop"

$steps = @(
    @{
        Name = "Check formatting"
        Command = "cargo"
        Args = @("fmt", "--check")
    },
    @{
        Name = "Run Clippy"
        Command = "cargo"
        Args = @("clippy", "--all-targets", "--all-features", "--", "-D", "warnings")
    },
    @{
        Name = "Run tests"
        Command = "cargo"
        Args = @("test")
    },
    @{
        Name = "Build"
        Command = "cargo"
        Args = @("build")
    }
)

foreach ($step in $steps) {
    Write-Host ""
    Write-Host "==> $($step.Name)"

    & $step.Command @($step.Args)

    if ($LASTEXITCODE -ne 0) {
        Write-Error "$($step.Name) failed with exit code $LASTEXITCODE."
        exit $LASTEXITCODE
    }
}

Write-Host ""
Write-Host "All local validation checks passed."
