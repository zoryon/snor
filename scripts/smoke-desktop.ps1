param(
    [string]$Executable = '.\src-tauri\target\release\snor.exe',
    [string]$OutputDirectory = '.\measurements\native-smoke'
)
$ErrorActionPreference = 'Stop'
$binaryPath = (Resolve-Path -LiteralPath $Executable).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
[void][System.IO.Directory]::CreateDirectory($outputRoot)
$readyPath = Join-Path $outputRoot ('ready-' + [Guid]::NewGuid().ToString('N') + '.json')
$previousReport = $env:SNOR_STARTUP_REPORT
$env:SNOR_STARTUP_REPORT = $readyPath
$timer = [System.Diagnostics.Stopwatch]::StartNew()
$appProcess = $null
try {
    $appProcess = Start-Process -FilePath $binaryPath -PassThru -WindowStyle Hidden
    while (-not (Test-Path -LiteralPath $readyPath)) {
        $appProcess.Refresh()
        if ($appProcess.HasExited) { throw ('App exited before UI readiness. Exit code: ' + $appProcess.ExitCode) }
        if ($timer.Elapsed.TotalSeconds -gt 30) { throw 'No frontend readiness signal within 30 seconds.' }
        Start-Sleep -Milliseconds 20
    }
    $timer.Stop()
    $native = Get-Content -LiteralPath $readyPath -Raw | ConvertFrom-Json
    $result = [pscustomobject]@{ executable = $binaryPath; externalLaunchToReadyMs = [math]::Round($timer.Elapsed.TotalMilliseconds,2); native = $native; aiEngineStarted = $false; note = 'Single local startup sample, hidden launch, frontend DOM readiness. Not a cold-start benchmark or full AI memory measurement.' }
    $result | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $outputRoot 'startup.json') -Encoding UTF8
    $result | ConvertTo-Json -Depth 5
    & (Join-Path $PSScriptRoot 'measure-process-tree.ps1') -RootProcessId $appProcess.Id -Samples 5 -IntervalMs 1000 -OutputPath (Join-Path $outputRoot 'memory.json')
} finally {
    $env:SNOR_STARTUP_REPORT = $previousReport
    if ($appProcess) {
        $appProcess.Refresh()
        if (-not $appProcess.HasExited) {
            [void]$appProcess.CloseMainWindow()
            if (-not $appProcess.WaitForExit(5000)) { $appProcess.Kill() }
        }
    }
}
