param(
    [Parameter(Mandatory = $true)]
    [string]$InputManifest,

    [Parameter(Mandatory = $true)]
    [string]$OutputDir,

    [Parameter(Mandatory = $true)]
    [string]$GoldenSetRevision,

    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-f]{64}$")]
    [string]$FontBundleSha256,

    [Parameter(Mandatory = $true)]
    [ValidateSet("disabled")]
    [string]$HostNetworkIsolation,

    [string]$WindowsVersion = "Unknown",
    [string]$OfficeChannel = "Unknown",
    [string]$PdfInfoPath = "pdfinfo",
    [string]$PdfToPpmPath = "pdftoppm",
    [int]$MaxSemanticCells = 1000000,
    [string]$CaptureTimestamp = (Get-Date).ToUniversalTime().ToString("o"),
    [string]$BatchId = ("office-" + [Guid]::NewGuid().ToString("N"))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

Import-Module (
    Join-Path (Split-Path -Parent $PSCommandPath) "multiformat/OfficeOracle.psm1"
) -Force

$supportedFormats = @("pptx", "docx", "doc", "xlsx", "xls", "ppt", "pdf")
$manifestPath = (Resolve-Path -LiteralPath $InputManifest).Path
$manifestRoot = Split-Path -Parent $manifestPath
$input = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($input.schema_version -ne 1) {
    throw "Input manifest schema_version must be 1"
}
$files = @($input.files)
if ($files.Count -eq 0) {
    throw "Input manifest must contain files"
}

if (Test-Path -LiteralPath $OutputDir) {
    $existing = @(Get-ChildItem -LiteralPath $OutputDir -Force)
    if ($existing.Count -ne 0) {
        throw "Output directory must be empty: $OutputDir"
    }
}
else {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}
$resolvedOutput = (Resolve-Path -LiteralPath $OutputDir).Path

$formats = @($files | ForEach-Object { ([string]$_.format).ToLowerInvariant() })
$word = $null
$wordDocuments = $null
$excel = $null
$excelWorkbooks = $null
$powerPoint = $null
$presentations = $null
$runtime = [ordered]@{
    windows = $WindowsVersion
    office_channel = $OfficeChannel
    word = $null
    excel = $null
    powerpoint = $null
}
$results = @()

try {
    if ($formats -contains "docx" -or $formats -contains "doc") {
        $word = New-Object -ComObject Word.Application
        $word.Visible = $false
        $word.DisplayAlerts = 0
        $word.AutomationSecurity = 3
        $word.Options.UpdateLinksAtOpen = $false
        $wordDocuments = $word.Documents
        $runtime.word = "$($word.Version).$($word.Build)"
    }
    if ($formats -contains "xlsx" -or $formats -contains "xls") {
        $excel = New-Object -ComObject Excel.Application
        $excel.Visible = $false
        $excel.DisplayAlerts = $false
        $excel.AskToUpdateLinks = $false
        $excel.AutomationSecurity = 3
        $excelWorkbooks = $excel.Workbooks
        $runtime.excel = "$($excel.Version).$($excel.Build)"
    }
    if ($formats -contains "pptx" -or $formats -contains "ppt") {
        $powerPoint = New-Object -ComObject PowerPoint.Application
        $powerPoint.Visible = 1
        $powerPoint.DisplayAlerts = 1
        $powerPoint.AutomationSecurity = 3
        $presentations = $powerPoint.Presentations
        $runtime.powerpoint = "$($powerPoint.Version).$($powerPoint.Build)"
    }

    foreach ($entry in $files) {
        $format = ([string]$entry.format).ToLowerInvariant()
        $id = [string]$entry.id
        if ($format -notin $supportedFormats) {
            throw "Unsupported format in input manifest: $format"
        }
        if ($id -notmatch "^[A-Za-z0-9][A-Za-z0-9._-]{0,127}$") {
            throw "Unsafe or empty file id: $id"
        }
        $sourcePath = Resolve-SafeCorpusPath -Root $manifestRoot -RelativePath $entry.path
        $sourceExtension = [System.IO.Path]::GetExtension($sourcePath).TrimStart(".").ToLowerInvariant()
        if ($sourceExtension -ne $format) {
            throw "Source extension does not match format for $id"
        }
        $itemOutput = Join-Path $resolvedOutput $id
        New-Item -ItemType Directory -Path $itemOutput | Out-Null
        $pdfPath = Join-Path $itemOutput "reference.pdf"
        $semantic = $null
        $directSlides = @()

        switch ($format) {
            { $_ -in @("docx", "doc") } {
                $document = $null
                try {
                    $document = $wordDocuments.Open($sourcePath, $false, $true, $false)
                    $document.ExportAsFixedFormat($pdfPath, 17)
                    $semantic = Get-WordSemanticInventory -Document $document
                }
                finally {
                    if ($null -ne $document) {
                        $document.Close(0)
                        Release-ComObject $document
                    }
                }
                break
            }
            { $_ -in @("xlsx", "xls") } {
                $workbook = $null
                try {
                    $workbook = $excelWorkbooks.Open($sourcePath, 0, $true)
                    $workbook.ExportAsFixedFormat(0, $pdfPath)
                    $semantic = Get-ExcelSemanticInventory `
                        -Workbook $workbook `
                        -MaxSemanticCells $MaxSemanticCells
                }
                finally {
                    if ($null -ne $workbook) {
                        $workbook.Close($false)
                        Release-ComObject $workbook
                    }
                }
                break
            }
            { $_ -in @("pptx", "ppt") } {
                $presentation = $null
                try {
                    $presentation = $presentations.Open($sourcePath, $true, $false, $false)
                    $presentation.ExportAsFixedFormat($pdfPath, 2)
                    for ($slideIndex = 1; $slideIndex -le $presentation.Slides.Count; $slideIndex++) {
                        $slide = $null
                        try {
                        $slide = $presentation.Slides.Item($slideIndex)
                        if ($slide.SlideShowTransition.Hidden -ne 0) {
                            continue
                        }
                        $slidePath = Join-Path $itemOutput ("slide-" + $slideIndex + ".png")
                            $slide.Export($slidePath, "PNG", 960, 540)
                            $dimensions = Get-PngDimensions -Path $slidePath
                            $directSlides += [ordered]@{
                                path = [System.IO.Path]::GetFileName($slidePath)
                                sha256 = Get-FileSha256 -Path $slidePath
                                width = $dimensions.width
                                height = $dimensions.height
                            }
                        }
                        finally {
                            Release-ComObject $slide
                        }
                    }
                    $semantic = Get-PowerPointSemanticInventory -Presentation $presentation
                }
                finally {
                    if ($null -ne $presentation) {
                        $presentation.Close()
                        Release-ComObject $presentation
                    }
                }
                break
            }
            "pdf" {
                Copy-Item -LiteralPath $sourcePath -Destination $pdfPath
                $semantic = [ordered]@{ source_is_normative = $true }
                break
            }
        }

        if (-not (Test-Path -LiteralPath $pdfPath -PathType Leaf)) {
            throw "Native export did not create a PDF for $id"
        }
        $pdfPageCount = Get-PdfPageCount -PdfInfoPath $PdfInfoPath -PdfPath $pdfPath
        $visualUnits = if ($format -in @("pptx", "ppt")) {
            $directSlides
        }
        else {
            @(
                Invoke-PdfRaster `
                    -PdfToPpmPath $PdfToPpmPath `
                    -PdfPath $pdfPath `
                    -OutputDir $itemOutput
            )
        }
        if ($visualUnits.Count -ne $pdfPageCount) {
            throw "Visual unit count differs from PDF page count for $id"
        }

        $semanticPath = Join-Path $itemOutput "semantic.json"
        Write-Utf8Json -Value $semantic -Path $semanticPath
        $results += [ordered]@{
            id = $id
            format = $format
            track = [string]$entry.track
            source_path = [string]$entry.path
            source_sha256 = Get-FileSha256 -Path $sourcePath
            pdf = [ordered]@{
                path = "$id/reference.pdf"
                sha256 = Get-FileSha256 -Path $pdfPath
                page_count = $pdfPageCount
            }
            semantic = [ordered]@{
                path = "$id/semantic.json"
                sha256 = Get-FileSha256 -Path $semanticPath
            }
            visual_units = $visualUnits
        }
    }
}
finally {
    if ($null -ne $wordDocuments) { Release-ComObject $wordDocuments }
    if ($null -ne $word) {
        $word.Quit()
        Release-ComObject $word
    }
    if ($null -ne $excelWorkbooks) { Release-ComObject $excelWorkbooks }
    if ($null -ne $excel) {
        $excel.Quit()
        Release-ComObject $excel
    }
    if ($null -ne $presentations) { Release-ComObject $presentations }
    if ($null -ne $powerPoint) {
        $powerPoint.Quit()
        Release-ComObject $powerPoint
    }
    [GC]::Collect()
    [GC]::WaitForPendingFinalizers()
}

$batch = [ordered]@{
    schema_version = 1
    batch_id = $BatchId
    capture_timestamp = $CaptureTimestamp
    golden_set_revision = $GoldenSetRevision
    font_bundle_sha256 = $FontBundleSha256
    network_isolation = $HostNetworkIsolation
    runtime = $runtime
    files = $results
}
Write-Utf8Json -Value $batch -Path (Join-Path $resolvedOutput "manifest.json")
