param([switch]$Purge)

$ErrorActionPreference = 'Stop'

$principal = [Security.Principal.WindowsPrincipal]::new([Security.Principal.WindowsIdentity]::GetCurrent())
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    $arguments = '-NoProfile -ExecutionPolicy Bypass -File "{0}"' -f $PSCommandPath
    if ($Purge) {
        $arguments += ' -Purge'
    }
    $process = Start-Process powershell.exe -Verb RunAs -ArgumentList $arguments -Wait -PassThru
    exit $process.ExitCode
}

. (Join-Path $PSScriptRoot 'common.ps1')
Stop-FEIghtstick

$destination = Join-Path $env:ProgramFiles 'FEIghtstick'
if (Get-ScheduledTask -TaskName 'FEIghtstick' -ErrorAction SilentlyContinue) {
    Unregister-ScheduledTask -TaskName 'FEIghtstick' -Confirm:$false
}
$shortcutPath = Join-Path ([Environment]::GetFolderPath('CommonPrograms')) 'FEIghtstick.lnk'
Remove-Item $shortcutPath -Force -ErrorAction SilentlyContinue
Remove-Item 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\FEIghtstick' -Recurse -Force -ErrorAction SilentlyContinue
foreach ($name in @('feightstick.exe', 'run.ps1', 'common.ps1', 'Start FEIghtstick.cmd', 'uninstall.ps1')) {
    $path = Join-Path $destination $name
    if (Test-Path $path) {
        Remove-Item $path -Force
    }
}

if ($Purge) {
    Remove-Item (Join-Path $destination 'profile.toml') -Force -ErrorAction SilentlyContinue
    Remove-Item (Join-Path $env:ProgramData 'FEIghtstick') -Recurse -Force -ErrorAction SilentlyContinue
}
if ((Test-Path $destination) -and -not (Get-ChildItem -LiteralPath $destination -Force)) {
    Remove-Item -LiteralPath $destination -Force
}

Write-Host 'FEIghtstick removed. Profiles and logs are preserved unless -Purge was supplied. ViGEmBus remains installed for other applications.'
