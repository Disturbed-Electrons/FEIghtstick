# Run after building from an elevated PowerShell; see README.md.
$ErrorActionPreference = 'Stop'

$identity = [Security.Principal.WindowsIdentity]::GetCurrent()
$principal = [Security.Principal.WindowsPrincipal]::new($identity)
if (-not $principal.IsInRole([Security.Principal.WindowsBuiltInRole]::Administrator)) {
    throw 'Open PowerShell as Administrator and run this installer again.'
}
. (Join-Path $PSScriptRoot 'common.ps1')

$root = (Resolve-Path (Join-Path $PSScriptRoot '..\..')).Path
$binary = Join-Path $root 'feightstick.exe'
if (-not (Test-Path $binary)) {
    $binary = Join-Path $root 'target\release\feightstick.exe'
}
if (-not (Test-Path $binary)) {
    throw 'Build first: cargo build --release'
}

$destination = Join-Path $env:ProgramFiles 'FEIghtstick'
$profile = Join-Path $destination 'profile.toml'
$sourceProfile = Join-Path $root 'profiles\default.toml'
if (Test-Path $profile) {
    $sourceProfile = $profile
}
& $binary check-profile $sourceProfile
if ($LASTEXITCODE -ne 0) {
    throw 'Fix the profile before installing.'
}
$version = (& $binary --version) -replace '^feightstick ', ''
if ($LASTEXITCODE -ne 0) {
    throw 'Could not read the executable version.'
}

if (-not (Test-Path 'HKLM:\SYSTEM\CurrentControlSet\Services\ViGEmBus')) {
    $installer = Join-Path ([IO.Path]::GetTempPath()) ('feightstick-vigem-' + [guid]::NewGuid() + '.exe')
    try {
        Invoke-WebRequest -UseBasicParsing 'https://github.com/nefarius/ViGEmBus/releases/download/v1.22.0/ViGEmBus_1.22.0_x64_x86_arm64.exe' -OutFile $installer
        $signature = Get-AuthenticodeSignature $installer
        if ($signature.Status -ne 'Valid' -or $signature.SignerCertificate.Subject -notmatch 'Nefarius') {
            throw 'ViGEmBus installer signature verification failed.'
        }
        $process = Start-Process $installer -ArgumentList '/quiet', '/norestart' -Wait -PassThru
        if ($process.ExitCode -notin @(0, 3010)) {
            throw "ViGEmBus installation failed: $($process.ExitCode)"
        }
        $restartRequired = $process.ExitCode -eq 3010
    } finally {
        Remove-Item $installer -ErrorAction SilentlyContinue
    }
}

Stop-FEIghtstick
New-Item -ItemType Directory -Force $destination | Out-Null
Copy-Item $binary (Join-Path $destination 'feightstick.exe') -Force
if (-not (Test-Path $profile)) {
    Copy-Item $sourceProfile $profile
}
foreach ($name in @('run.ps1', 'common.ps1', 'uninstall.ps1', 'Start FEIghtstick.cmd')) {
    Copy-Item (Join-Path $PSScriptRoot $name) $destination -Force
}

$stateDirectory = Join-Path $env:ProgramData 'FEIghtstick'
New-Item -ItemType Directory -Force $stateDirectory | Out-Null
$acl = Get-Acl $stateDirectory
$rule = [Security.AccessControl.FileSystemAccessRule]::new(
    $identity.User,
    'Modify',
    'ContainerInherit,ObjectInherit',
    'None',
    'Allow'
)
$acl.SetAccessRule($rule)
Set-Acl $stateDirectory $acl
Remove-Item (Join-Path $stateDirectory 'stop') -Force -ErrorAction SilentlyContinue

$powerShell = Join-Path $env:SystemRoot 'System32\WindowsPowerShell\v1.0\powershell.exe'
$arguments = '-NoProfile -NonInteractive -WindowStyle Hidden -ExecutionPolicy Bypass -File "{0}"' -f (Join-Path $destination 'run.ps1')
$action = New-ScheduledTaskAction -Execute $powerShell -Argument $arguments -WorkingDirectory $destination
$trigger = New-ScheduledTaskTrigger -AtLogOn -User $identity.Name
$taskPrincipal = New-ScheduledTaskPrincipal -UserId $identity.Name -LogonType Interactive -RunLevel Limited
$settingsOptions = @{
    ExecutionTimeLimit = [TimeSpan]::Zero
    MultipleInstances = 'IgnoreNew'
    RestartCount = 3
    RestartInterval = New-TimeSpan -Minutes 1
    AllowStartIfOnBatteries = $true
    DontStopIfGoingOnBatteries = $true
}
$settings = New-ScheduledTaskSettingsSet @settingsOptions
Register-ScheduledTask -TaskName 'FEIghtstick' -Action $action -Trigger $trigger -Principal $taskPrincipal -Settings $settings -Force | Out-Null

$shortcutPath = Join-Path ([Environment]::GetFolderPath('CommonPrograms')) 'FEIghtstick.lnk'
$shell = New-Object -ComObject WScript.Shell
$shortcut = $shell.CreateShortcut($shortcutPath)
$shortcut.TargetPath = $powerShell
$shortcut.Arguments = '-NoProfile -NonInteractive -WindowStyle Hidden -Command "Start-ScheduledTask -TaskName FEIghtstick"'
$shortcut.WindowStyle = 7
$shortcut.Save()

$uninstallKey = 'HKLM:\SOFTWARE\Microsoft\Windows\CurrentVersion\Uninstall\FEIghtstick'
New-Item $uninstallKey -Force | Out-Null
$properties = @{
    DisplayName = 'FEIghtstick'
    DisplayVersion = $version
    Publisher = 'FEIghtstick'
    InstallLocation = $destination
    UninstallString = '"{0}" -NoProfile -ExecutionPolicy Bypass -File "{1}"' -f $powerShell, (Join-Path $destination 'uninstall.ps1')
}
foreach ($entry in $properties.GetEnumerator()) {
    New-ItemProperty $uninstallKey -Name $entry.Key -Value $entry.Value -PropertyType String -Force | Out-Null
}

Start-ScheduledTask -TaskName 'FEIghtstick'
Write-Host "Installed. FEIghtstick is configured to start now and in the background when $($identity.Name) signs in. Keep the joystick released when connecting."
if ($restartRequired) {
    Write-Host 'Restart Windows to finish installing ViGEmBus.'
}
