param(
    [Parameter(Mandatory=$true)][int]$TargetProcessId,
    [ValidateSet('List','Click','SetValue','Close')][string]$Action='List',
    [string]$Name,
    [string]$Value
)
$ErrorActionPreference='Stop'
Add-Type -AssemblyName UIAutomationClient,UIAutomationTypes
if(-not ('FileNymNativeControl' -as [type])){Add-Type -Path (Join-Path $PSScriptRoot 'native-controls.cs')}
$process=Get-Process -Id $TargetProcessId
if($process.ProcessName -ne 'filenym-desktop'){throw 'Only a specifically identified FileNym process may be driven'}
$condition=New-Object System.Windows.Automation.PropertyCondition([System.Windows.Automation.AutomationElement]::ProcessIdProperty,$TargetProcessId)
$windows=[System.Windows.Automation.AutomationElement]::RootElement.FindAll([System.Windows.Automation.TreeScope]::Children,$condition)
$elements=New-Object 'System.Collections.Generic.List[System.Windows.Automation.AutomationElement]'
foreach($window in $windows){
    $elements.Add($window)
    foreach($element in $window.FindAll([System.Windows.Automation.TreeScope]::Descendants,[System.Windows.Automation.Condition]::TrueCondition)){$elements.Add($element)}
}
if($Action -eq 'List'){
    $elements | ForEach-Object {
        $pattern=$null;$text=$null
        if($_.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern,[ref]$pattern)){$text=$pattern.Current.Value}
        [PSCustomObject]@{Name=$_.Current.Name;Id=$_.Current.AutomationId;Type=$_.Current.ControlType.ProgrammaticName;Enabled=$_.Current.IsEnabled;Value=$text;Focused=$_.Current.HasKeyboardFocus}
    } | ConvertTo-Json -Depth 3
    return
}
if($Action -eq 'Close'){
    $candidates=@($windows | Where-Object {$_.Current.Name -eq 'FileNym'})
    if($candidates.Count -ne 1){throw 'Expected one titled test window'}
    $candidates[0].GetCurrentPattern([System.Windows.Automation.WindowPattern]::Pattern).Close()
    if(-not $process.WaitForExit(15000)){throw 'Test app did not drain/close within the acceptance deadline'}
    Write-Output 'PASS: test window closed cleanly'
    return
}
$candidates=@($elements | Where-Object {
    $_.Current.IsEnabled -and $(if($Action -eq 'SetValue' -and $Name -eq 'File name:'){
        $_.Current.AutomationId -eq '1148' -and [FileNymNativeControl]::Class([IntPtr]$_.Current.NativeWindowHandle) -eq 'Edit'
    }else{$_.Current.Name -eq $Name})
} | Where-Object {
    if($Action -eq 'Click'){
        $pattern=$null
        $_.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern,[ref]$pattern) -or $_.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern,[ref]$pattern) -or [FileNymNativeControl]::Class([IntPtr]$_.Current.NativeWindowHandle) -eq 'Button'
    }else{
        $pattern=$null
        $_.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern,[ref]$pattern) -or [FileNymNativeControl]::Class([IntPtr]$_.Current.NativeWindowHandle) -eq 'Edit'
    }
})
if($candidates.Count -ne 1){throw "Expected one enabled actionable control named '$Name'; found $($candidates.Count)"}
$target=$candidates[0];$pattern=$null
if($Action -eq 'Click'){
    if($target.TryGetCurrentPattern([System.Windows.Automation.InvokePattern]::Pattern,[ref]$pattern)){$pattern.Invoke()}
    elseif($target.TryGetCurrentPattern([System.Windows.Automation.ExpandCollapsePattern]::Pattern,[ref]$pattern)){if($pattern.Current.ExpandCollapseState -eq [System.Windows.Automation.ExpandCollapseState]::Collapsed){$pattern.Expand()}else{$pattern.Collapse()}}
    else{[FileNymNativeControl]::Click([IntPtr]$target.Current.NativeWindowHandle,$TargetProcessId)}
}
if($Action -eq 'SetValue'){
    if($target.TryGetCurrentPattern([System.Windows.Automation.ValuePattern]::Pattern,[ref]$pattern)){$pattern.SetValue($Value)}
    else{[FileNymNativeControl]::SetText([IntPtr]$target.Current.NativeWindowHandle,$TargetProcessId,$Value)}
}
