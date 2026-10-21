param(
    [string]$Executable = '.\src-tauri\target\release\snor.exe',
    [ValidateRange(1, 30)][int]$Runs = 5,
    [string]$OutputDirectory = '.\measurements\startup'
)
$ErrorActionPreference = 'Stop'
$binaryPath = (Resolve-Path -LiteralPath $Executable).Path
$outputRoot = [System.IO.Path]::GetFullPath($OutputDirectory)
[void][System.IO.Directory]::CreateDirectory($outputRoot)
$previousReport = $env:SNOR_STARTUP_REPORT
$samples = [System.Collections.Generic.List[object]]::new()
try {
    for ($runIndex = 0; $runIndex -lt $Runs; $runIndex++) {
        $readyPath = Join-Path $outputRoot ('ready-' + [Guid]::NewGuid().ToString('N') + '.json')
        $env:SNOR_STARTUP_REPORT = $readyPath
        $appProcess = $null
        $timer = [System.Diagnostics.Stopwatch]::StartNew()
        try {
            $appProcess = Start-Process -FilePath $binaryPath -PassThru -WindowStyle Hidden
            while (-not (Test-Path -LiteralPath $readyPath)) {
                $appProcess.Refresh()
                if ($appProcess.HasExited) { throw ('App exited early with code ' + $appProcess.ExitCode) }
                if ($timer.Elapsed.TotalSeconds -gt 30) { throw 'UI readiness timed out.' }
                Start-Sleep -Milliseconds 20
            }
            $timer.Stop()
            $native = Get-Content -LiteralPath $readyPath -Raw | ConvertFrom-Json
            $samples.Add([pscustomobject]@{ run = ($runIndex+1); externalLaunchToReadyMs = [math]::Round($timer.Elapsed.TotalMilliseconds,2); native = $native })
            Write-Output ('Run {0}: external {1} ms; Rust entry to DOM ready {2:N2} ms' -f ($runIndex+1), $timer.Elapsed.TotalMilliseconds, $native.backendToFrontendReadyMs)
        } finally {
            if ($appProcess) {
                $appProcess.Refresh()
                if (-not $appProcess.HasExited) { [void]$appProcess.CloseMainWindow(); if (-not $appProcess.WaitForExit(5000)) { $appProcess.Kill() } }
            }
        }
    }
} finally { $env:SNOR_STARTUP_REPORT = $previousReport }
$ordered = @($samples | ForEach-Object { $_.externalLaunchToReadyMs } | Sort-Object)
$median = if ($ordered.Count % 2) { $ordered[[int][math]::Floor($ordered.Count/2)] } else { ($ordered[$ordered.Count/2-1]+$ordered[$ordered.Count/2])/2 }
$p95 = $ordered[[math]::Max(0,[math]::Ceiling($ordered.Count * 0.95)-1)]
$result = [pscustomobject]@{ utc = [DateTime]::UtcNow.ToString('o'); osVersion = [Environment]::OSVersion.VersionString; processorCount = [Environment]::ProcessorCount; runs = $Runs; medianExternalMs = $median; p95ExternalMs = $p95; note = 'Repeated process launches with existing WebView2 profile and warm OS cache; not resident-window recall or a controlled cold-start benchmark. AI engine not started.'; samples = $samples }
$result | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $outputRoot 'summary.json') -Encoding UTF8
Write-Output ('Median {0:N2} ms; p95 {1:N2} ms. Report: {2}' -f $median,$p95,(Join-Path $outputRoot 'summary.json'))
