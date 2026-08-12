param(
    [Parameter(Mandatory = $true)]
    [string]$InputDir,

    [Parameter(Mandatory = $true)]
    [string]$OutputDir,

    [string]$PowerPointChannel = "Manual",

    [string]$WindowsVersion = "Unknown",

    [string]$OutputResolution = "Unknown",

    [Parameter(Mandatory = $true)]
    [string]$GoldenSetRevision,

    [string]$CaptureTimestamp = (Get-Date).ToUniversalTime().ToString("o"),

    [string]$BatchId = ("powerpoint-" + [Guid]::NewGuid().ToString("N"))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Release-ComObject {
    param([object]$ComObject)

    if ($null -ne $ComObject) {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($ComObject)
    }
}

$resolvedInput = Resolve-Path -Path $InputDir
if (-not (Test-Path -Path $resolvedInput -PathType Container)) {
    throw "Input directory does not exist: $InputDir"
}

if (-not (Test-Path -Path $OutputDir)) {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}

$powerPoint = $null
$presentations = $null
$scriptRoot = Split-Path -Parent $PSCommandPath
$resolvedOutput = Resolve-Path -Path $OutputDir
$powerPointVersion = $null
$powerPointBuild = $null

try {
    $powerPoint = New-Object -ComObject PowerPoint.Application
    $powerPoint.Visible = 1
    $powerPointVersion = $powerPoint.Version
    $powerPointBuild = "$($powerPoint.Version).$($powerPoint.Build)"
    $presentations = $powerPoint.Presentations

    Get-ChildItem -Path $resolvedInput -Filter *.pptx | Sort-Object Name | ForEach-Object {
        $deck = $null
        $deckName = [System.IO.Path]::GetFileNameWithoutExtension($_.Name)
        $deckOutput = Join-Path $OutputDir $deckName

        if (-not (Test-Path -Path $deckOutput)) {
            New-Item -ItemType Directory -Path $deckOutput | Out-Null
        }

        try {
            $deck = $presentations.Open($_.FullName, $false, $true, $false)
            $deck.SaveAs($deckOutput, 18)
        }
        finally {
            if ($null -ne $deck) {
                $deck.Close()
                Release-ComObject $deck
            }
        }
    }
}
finally {
    if ($null -ne $presentations) {
        Release-ComObject $presentations
    }
    if ($null -ne $powerPoint) {
        $powerPoint.Quit()
        Release-ComObject $powerPoint
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}

$python = Get-Command python -ErrorAction SilentlyContinue
if ($null -eq $python) {
    $python = Get-Command py -ErrorAction SilentlyContinue
}

if ($null -eq $python) {
    throw "Python is required to scaffold metadata.json and manifest.json"
}

$exportCommand = "pwsh -File ./reference_render_powerpoint.ps1 -InputDir ./golden_set -OutputDir ./powerpoint_golden"
& $python.Source (Join-Path $scriptRoot "scaffold_powerpoint_golden_batch.py") `
    --golden-set-dir $resolvedInput `
    --output-dir $resolvedOutput `
    --powerpoint-version $powerPointVersion `
    --powerpoint-build $powerPointBuild `
    --powerpoint-channel $PowerPointChannel `
    --windows-version $WindowsVersion `
    --export-command $exportCommand `
    --output-resolution $OutputResolution `
    --golden-set-revision $GoldenSetRevision `
    --capture-timestamp $CaptureTimestamp `
    --batch-id $BatchId
