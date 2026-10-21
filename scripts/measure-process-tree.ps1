param(
    [Parameter(Mandatory = $true)][int]$RootProcessId,
    [ValidateRange(1, 3600)][int]$Samples = 30,
    [ValidateRange(100, 60000)][int]$IntervalMs = 1000,
    [string]$OutputPath = '.\measurements\process-tree.json'
)
$ErrorActionPreference = 'Stop'
$report = [System.Collections.Generic.List[object]]::new()
$knownIds = [System.Collections.Generic.HashSet[int]]::new()
[void]$knownIds.Add($RootProcessId)
for ($sampleIndex = 0; $sampleIndex -lt $Samples; $sampleIndex++) {
    $processList = @(Get-CimInstance Win32_Process | Select-Object ProcessId, ParentProcessId, Name)
    $changed = $true
    while ($changed) {
        $changed = $false
        foreach ($item in $processList) {
            if ($knownIds.Contains([int]$item.ParentProcessId) -and $knownIds.Add([int]$item.ProcessId)) { $changed = $true }
        }
    }
    $rows = @()
    $residentCounters = @{}
    try { foreach ($counter in (Get-CimInstance Win32_PerfFormattedData_PerfProc_Process -ErrorAction Stop)) { $residentCounters[[int]$counter.IDProcess] = [long]$counter.WorkingSetPrivate } } catch { }
    foreach ($item in $processList) {
        if ($knownIds.Contains([int]$item.ProcessId)) {
            $process = Get-Process -Id $item.ProcessId -ErrorAction SilentlyContinue
            if ($process) { $rows += [pscustomobject]@{ id = $process.Id; parentId = $item.ParentProcessId; name = $item.Name; workingSetBytes = $process.WorkingSet64; privateBytes = $process.PrivateMemorySize64; privateWorkingSetBytes = $(if ($residentCounters.ContainsKey($process.Id)) { $residentCounters[$process.Id] } else { $null }) } }
        }
    }
    $workingSet = ($rows | Measure-Object workingSetBytes -Sum).Sum
    $privateBytes = ($rows | Measure-Object privateBytes -Sum).Sum
    $privateResidentMiB = if (@($rows | Where-Object { $null -eq $_.privateWorkingSetBytes }).Count -eq 0) { [math]::Round(($rows | Measure-Object privateWorkingSetBytes -Sum).Sum / 1MB,2) } else { $null }
    $report.Add([pscustomobject]@{ utc = [DateTime]::UtcNow.ToString('o'); workingSetMiB = [math]::Round($workingSet / 1MB, 2); privateMiB = [math]::Round($privateBytes / 1MB, 2); privateWorkingSetMiB = $privateResidentMiB; processes = $rows })
    Write-Output ('Sample {0}/{1}: working set {2} MiB; private {3} MiB; {4} processes' -f ($sampleIndex+1), $Samples, [math]::Round($workingSet/1MB,2), [math]::Round($privateBytes/1MB,2), $rows.Count)
    if ($sampleIndex -lt ($Samples - 1)) { Start-Sleep -Milliseconds $IntervalMs }
}
$resolvedOutput = [System.IO.Path]::GetFullPath($OutputPath)
$parentDirectory = [System.IO.Path]::GetDirectoryName($resolvedOutput)
[void][System.IO.Directory]::CreateDirectory($parentDirectory)
$document = [pscustomobject]@{ rootProcessId = $RootProcessId; intervalMs = $IntervalMs; metricNote = 'Summed process working sets can double-count shared pages. Includes discovered descendants, not unrelated or reparented processes.'; samples = $report }
[System.IO.File]::WriteAllText($resolvedOutput, ($document | ConvertTo-Json -Depth 8), [System.Text.UTF8Encoding]::new($false))
Write-Output ('Report: {0}' -f $resolvedOutput)
