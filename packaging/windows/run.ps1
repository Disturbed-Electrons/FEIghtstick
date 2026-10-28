$ErrorActionPreference = 'Stop'

$stateDirectory = Join-Path $env:ProgramData 'FEIghtstick'
$stopFile = Join-Path $stateDirectory 'stop'

$processOptions = @{
    FilePath = Join-Path $PSScriptRoot 'feightstick.exe'
    ArgumentList = 'run --center-on-connect --profile "{0}" --shutdown-file "{1}"' -f (Join-Path $PSScriptRoot 'profile.toml'), $stopFile
    WorkingDirectory = $PSScriptRoot
    NoNewWindow = $true
    Wait = $true
    PassThru = $true
    RedirectStandardOutput = Join-Path $stateDirectory 'output.log'
    RedirectStandardError = Join-Path $stateDirectory 'feightstick.log'
}

$process = Start-Process @processOptions
exit $process.ExitCode
