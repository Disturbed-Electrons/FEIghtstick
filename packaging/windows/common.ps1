function Stop-FEIghtstick {
    $task = Get-ScheduledTask -TaskName 'FEIghtstick' -ErrorAction SilentlyContinue
    if (-not $task) {
        return
    }

    $stateDirectory = Join-Path $env:ProgramData 'FEIghtstick'
    New-Item -ItemType Directory -Force $stateDirectory | Out-Null
    New-Item -ItemType File -Force (Join-Path $stateDirectory 'stop') | Out-Null

    $deadline = [DateTime]::UtcNow.AddSeconds(10)
    while ((Get-ScheduledTask -TaskName 'FEIghtstick').State -in @('Running', 'Queued')) {
        if ([DateTime]::UtcNow -ge $deadline) {
            throw 'FEIghtstick did not stop. Restart Windows before retrying.'
        }
        Start-Sleep -Milliseconds 100
    }

    Disable-ScheduledTask -TaskName 'FEIghtstick' | Out-Null
    Remove-Item (Join-Path $stateDirectory 'stop') -Force
}
