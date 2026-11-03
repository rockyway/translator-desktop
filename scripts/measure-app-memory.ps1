# Read-only census of the supplied host and its WebView2 descendants. No profile access.
[CmdletBinding()]
param([Parameter(Mandatory = $true)][ValidateRange(1, 2147483647)][int]$RootPid)
$ErrorActionPreference = 'Stop'
$hostProcess = Get-Process -Id $RootPid -ErrorAction Stop
$all = @(Get-CimInstance Win32_Process)
$owned = [System.Collections.Generic.HashSet[int]]::new()
[void]$owned.Add($RootPid)
# Walk through intermediate descendants too, but report only the host, WebView2, and text-helper.
do {
    $added = $false
    foreach ($process in $all) {
        if ($owned.Contains([int]$process.ParentProcessId) -and $owned.Add([int]$process.ProcessId)) {
            $added = $true
        }
    }
} while ($added)
$rows = @(
    foreach ($process in $all) {
        $processId = [int]$process.ProcessId
        if (-not $owned.Contains($processId)) { continue }
        if ($processId -ne $RootPid -and $process.Name -ine 'msedgewebview2.exe' -and $process.Name -ine 'text-helper.exe') { continue }
        $role = 'unknown'
        if ($processId -eq $RootPid) { $role = 'host' }
        elseif ($process.Name -ieq 'text-helper.exe') { $role = 'sidecar' }
        elseif ($null -ne $process.CommandLine) {
            if ($process.CommandLine -match '--type=([^\s"]+)') {
                $role = switch ($Matches[1]) {
                    'renderer' { 'renderer' }
                    'gpu-process' { 'gpu' }
                    'crashpad-handler' { 'crashpad' }
                    'utility' {
                        if ($process.CommandLine -match 'network\.mojom') { 'network' }
                        elseif ($process.CommandLine -match 'storage\.mojom') { 'storage' }
                        else { 'utility' }
                    }
                    default { 'other' }
                }
            }
            else { $role = 'browser' }
        }
        # Processes can exit or deny access between the snapshot and this read.
        try {
            $live = Get-Process -Id $processId -ErrorAction Stop
            # Refuse PID reuse since the CIM snapshot.
            if ([math]::Abs(($live.StartTime.ToUniversalTime() - $process.CreationDate.ToUniversalTime()).TotalMilliseconds) -gt 1) { continue }
            $workingSetBytes = [long]$live.WorkingSet64
            $privateBytes = [long]$live.PrivateMemorySize64
        }
        catch { continue }
        [pscustomobject]@{
            Pid = $processId
            Role = $role
            WorkingSetBytes = $workingSetBytes
            PrivateBytes = $privateBytes
        }
    }
)
$rows | Select-Object Pid, Role,
    @{n='WorkingSetMB'; e={[math]::Round($_.WorkingSetBytes / 1MB, 2)}},
    @{n='PrivateMB'; e={[math]::Round($_.PrivateBytes / 1MB, 2)}} | Format-Table -AutoSize
$workingSet = ($rows | Measure-Object WorkingSetBytes -Sum).Sum
$private = ($rows | Measure-Object PrivateBytes -Sum).Sum
'Total: {0} processes; working set {1:N2} MB; private {2:N2} MB' -f $rows.Count, ($workingSet / 1MB), ($private / 1MB)
'Working set includes shared pages counted per process. Private bytes are committed memory, not residency.'
