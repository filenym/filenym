param([Parameter(Mandatory=$true)][int]$TargetProcessId,[Parameter(Mandatory=$true)][string]$FixturePath)
$ErrorActionPreference='Stop'
$helper=Join-Path $PSScriptRoot 'smoke.ps1'
$fixture=[IO.Path]::GetFullPath($FixturePath)
$owned=[IO.Path]::GetFullPath((Join-Path $PSScriptRoot '../../temp'))+[IO.Path]::DirectorySeparatorChar
if(-not $fixture.StartsWith($owned,[StringComparison]::OrdinalIgnoreCase) -or -not (Test-Path -LiteralPath $fixture -PathType Leaf)){throw 'Fixture must be an existing owned file beneath repository temp'}
function Action([string]$kind,[string]$name,[string]$value=''){
 $deadline=[DateTime]::UtcNow.AddSeconds(12)
 do{try{& $helper -TargetProcessId $TargetProcessId -Action $kind -Name $name -Value $value;return}catch{if($_.Exception.Message -notlike 'Expected one enabled actionable*found 0'){throw};Start-Sleep -Milliseconds 100}}while([DateTime]::UtcNow -lt $deadline)
 throw "Control unavailable before dispatch: $name"
}
function Assert-Text([string]$text){$deadline=[DateTime]::UtcNow.AddSeconds(12);do{$items=(& $helper -TargetProcessId $TargetProcessId -Action List|ConvertFrom-Json);if(@($items|Where-Object Name -eq $text).Count -gt 0){Write-Output "PASS: $text";return};Start-Sleep -Milliseconds 100}while([DateTime]::UtcNow -lt $deadline);throw "Missing expected text: $text"}
function Pick{Action Click '+ Add file';Action SetValue 'File name:' $fixture;Action Click Open}
Assert-Text '0 registered'
Action Click '+ Add file';Action Click Cancel;Assert-Text '0 registered'
Pick;Assert-Text '1 registered';Assert-Text 'Saved location'
foreach($name in @('Context.PDF','Second.txt','Third.txt')){Action Click '+ Add';Action SetValue 'Alias name' $name;Action Click Save;Assert-Text $name}
Action Click 'Actions for Context.PDF';Action Click Rename;Action SetValue 'Alias name' 'context.PDF';Action Click Save;Assert-Text 'context.PDF'
Action Click 'Actions for context.PDF';Action Click Rename;Action Click Save;Assert-Text 'context.PDF'
Action Click 'Actions for Second.txt';Action Click Delete;Assert-Text 'Delete alias?';Action Click Cancel;Assert-Text 'Second.txt'
Action Click Delete;Action Click 'Delete alias';Assert-Text 'Aliases 2'
foreach($decision in @('Cancel','View existing 1','Add separately')){Pick;Assert-Text 'Already registered location';Action Click $decision;Assert-Text 'Saved location'}
Assert-Text '2 registered'
Write-Output 'PASS native picker cancellation, selection, CRUD and all duplicate decisions'
