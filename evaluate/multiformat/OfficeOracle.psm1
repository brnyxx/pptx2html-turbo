Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Release-ComObject {
    param([object]$ComObject)

    if ($null -ne $ComObject) {
        [void][System.Runtime.InteropServices.Marshal]::ReleaseComObject($ComObject)
    }
}

function Write-Utf8Json {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Value,

        [Parameter(Mandatory = $true)]
        [string]$Path
    )

    $json = $Value | ConvertTo-Json -Depth 30 -Compress
    [System.IO.File]::WriteAllText(
        $Path,
        $json + [Environment]::NewLine,
        [System.Text.UTF8Encoding]::new($false)
    )
}

function Get-FileSha256 {
    param([Parameter(Mandatory = $true)][string]$Path)

    return (Get-FileHash -Algorithm SHA256 -LiteralPath $Path).Hash.ToLowerInvariant()
}

function Resolve-SafeCorpusPath {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Root,

        [Parameter(Mandatory = $true)]
        [string]$RelativePath
    )

    if ([System.IO.Path]::IsPathRooted($RelativePath)) {
        throw "Corpus path must be relative: $RelativePath"
    }
    $resolvedRoot = [System.IO.Path]::GetFullPath($Root)
    $resolved = [System.IO.Path]::GetFullPath((Join-Path $resolvedRoot $RelativePath))
    $rootPrefix = $resolvedRoot.TrimEnd(
        [System.IO.Path]::DirectorySeparatorChar
    ) + [System.IO.Path]::DirectorySeparatorChar
    if (-not $resolved.StartsWith($rootPrefix, [System.StringComparison]::OrdinalIgnoreCase)) {
        throw "Corpus path escapes the manifest root: $RelativePath"
    }
    if (-not (Test-Path -LiteralPath $resolved -PathType Leaf)) {
        throw "Corpus file does not exist: $RelativePath"
    }
    return $resolved
}

function Get-PngDimensions {
    param([Parameter(Mandatory = $true)][string]$Path)

    $bytes = [System.IO.File]::ReadAllBytes($Path)
    if ($bytes.Length -lt 24) {
        throw "PNG is truncated: $Path"
    }
    $signature = [byte[]](137, 80, 78, 71, 13, 10, 26, 10)
    for ($index = 0; $index -lt $signature.Length; $index++) {
        if ($bytes[$index] -ne $signature[$index]) {
            throw "Invalid PNG signature: $Path"
        }
    }
    $width = [System.Net.IPAddress]::NetworkToHostOrder(
        [BitConverter]::ToInt32($bytes, 16)
    )
    $height = [System.Net.IPAddress]::NetworkToHostOrder(
        [BitConverter]::ToInt32($bytes, 20)
    )
    return [ordered]@{ width = $width; height = $height }
}

function Get-PdfPageCount {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PdfInfoPath,

        [Parameter(Mandatory = $true)]
        [string]$PdfPath
    )

    $output = & $PdfInfoPath $PdfPath
    if ($LASTEXITCODE -ne 0) {
        throw "pdfinfo failed for $PdfPath"
    }
    $pageLines = @(
        $output | Where-Object { $_ -match "^Pages:\s+(?<Count>[1-9][0-9]*)\s*$" }
    )
    if ($pageLines.Count -ne 1) {
        throw "pdfinfo must emit exactly one positive Pages field for $PdfPath"
    }
    [void]($pageLines[0] -match "^Pages:\s+(?<Count>[1-9][0-9]*)\s*$")
    return [int]$Matches["Count"]
}

function Invoke-PdfRaster {
    param(
        [Parameter(Mandatory = $true)]
        [string]$PdfToPpmPath,

        [Parameter(Mandatory = $true)]
        [string]$PdfPath,

        [Parameter(Mandatory = $true)]
        [string]$OutputDir
    )

    $prefix = Join-Path $OutputDir "page"
    & $PdfToPpmPath "-png" "-r" "144" $PdfPath $prefix
    if ($LASTEXITCODE -ne 0) {
        throw "pdftoppm failed for $PdfPath"
    }
    $pages = @(
        Get-ChildItem -LiteralPath $OutputDir -Filter "page-*.png" |
            Sort-Object { [int]($_.BaseName -replace "^page-", "") }
    )
    if ($pages.Count -eq 0) {
        throw "pdftoppm emitted no page images for $PdfPath"
    }
    return @(
        $pages | ForEach-Object {
            $dimensions = Get-PngDimensions -Path $_.FullName
            [ordered]@{
                path = $_.Name
                sha256 = Get-FileSha256 -Path $_.FullName
                width = $dimensions.width
                height = $dimensions.height
            }
        }
    )
}

function Get-WordSemanticInventory {
    param([Parameter(Mandatory = $true)][object]$Document)

    $content = $null
    try {
        $content = $Document.Content
        $headerFooterCount = 0
        for ($sectionIndex = 1; $sectionIndex -le $Document.Sections.Count; $sectionIndex++) {
            $section = $null
            try {
                $section = $Document.Sections.Item($sectionIndex)
                $headerFooterCount += $section.Headers.Count + $section.Footers.Count
            }
            finally {
                Release-ComObject $section
            }
        }
        $links = @()
        for ($linkIndex = 1; $linkIndex -le $Document.Hyperlinks.Count; $linkIndex++) {
            $link = $null
            try {
                $link = $Document.Hyperlinks.Item($linkIndex)
                $links += [ordered]@{
                    address = [string]$link.Address
                    sub_address = [string]$link.SubAddress
                    text = [string]$link.TextToDisplay
                }
            }
            finally {
                Release-ComObject $link
            }
        }
        return [ordered]@{
            ordered_text = [string]$content.Text
            native_page_count = [int]$Document.ComputeStatistics(2)
            table_count = [int]$Document.Tables.Count
            inline_shape_count = [int]$Document.InlineShapes.Count
            shape_count = [int]$Document.Shapes.Count
            header_footer_count = $headerFooterCount
            footnote_count = [int]$Document.Footnotes.Count
            endnote_count = [int]$Document.Endnotes.Count
            hyperlinks = $links
        }
    }
    finally {
        Release-ComObject $content
    }
}

function Get-ExcelSemanticInventory {
    param(
        [Parameter(Mandatory = $true)]
        [object]$Workbook,

        [Parameter(Mandatory = $true)]
        [int]$MaxSemanticCells
    )

    $worksheets = @()
    for ($sheetIndex = 1; $sheetIndex -le $Workbook.Worksheets.Count; $sheetIndex++) {
        $sheet = $null
        $usedRange = $null
        try {
            $sheet = $Workbook.Worksheets.Item($sheetIndex)
            $usedRange = $sheet.UsedRange
            $cellCount = [long]$usedRange.Rows.Count * [long]$usedRange.Columns.Count
            if ($cellCount -gt $MaxSemanticCells) {
                throw "Worksheet $($sheet.Name) exceeds MaxSemanticCells=$MaxSemanticCells"
            }
            $cells = @()
            for ($row = 1; $row -le $usedRange.Rows.Count; $row++) {
                for ($column = 1; $column -le $usedRange.Columns.Count; $column++) {
                    $cell = $null
                    try {
                        $cell = $usedRange.Cells.Item($row, $column)
                        $display = [string]$cell.Text
                        $formula = [string]$cell.Formula
                        if ($display.Length -gt 0 -or $formula.Length -gt 0) {
                            $cells += [ordered]@{
                                address = [string]$cell.Address
                                display = $display
                                formula = $formula
                                number_format = [string]$cell.NumberFormat
                            }
                        }
                    }
                    finally {
                        Release-ComObject $cell
                    }
                }
            }
            $worksheets += [ordered]@{
                index = $sheetIndex
                name = [string]$sheet.Name
                visible = [int]$sheet.Visible
                print_area = [string]$sheet.PageSetup.PrintArea
                used_range = [string]$usedRange.Address
                chart_count = [int]$sheet.ChartObjects().Count
                shape_count = [int]$sheet.Shapes.Count
                cells = $cells
            }
        }
        finally {
            Release-ComObject $usedRange
            Release-ComObject $sheet
        }
    }
    return [ordered]@{ worksheets = $worksheets }
}

function Get-PowerPointSemanticInventory {
    param([Parameter(Mandatory = $true)][object]$Presentation)

    $slides = @()
    for ($slideIndex = 1; $slideIndex -le $Presentation.Slides.Count; $slideIndex++) {
        $slide = $null
        try {
            $slide = $Presentation.Slides.Item($slideIndex)
            $shapes = @()
            for ($shapeIndex = 1; $shapeIndex -le $slide.Shapes.Count; $shapeIndex++) {
                $shape = $null
                try {
                    $shape = $slide.Shapes.Item($shapeIndex)
                    $text = ""
                    if ($shape.HasTextFrame -ne 0 -and $shape.TextFrame.HasText -ne 0) {
                        $text = [string]$shape.TextFrame.TextRange.Text
                    }
                    $shapes += [ordered]@{
                        id = [int]$shape.Id
                        name = [string]$shape.Name
                        type = [int]$shape.Type
                        text = $text
                    }
                }
                finally {
                    Release-ComObject $shape
                }
            }
            $slides += [ordered]@{
                index = $slideIndex
                hidden = ($slide.SlideShowTransition.Hidden -ne 0)
                shapes = $shapes
            }
        }
        finally {
            Release-ComObject $slide
        }
    }
    return [ordered]@{ slides = $slides }
}

Export-ModuleMember -Function @(
    "Release-ComObject",
    "Write-Utf8Json",
    "Get-FileSha256",
    "Resolve-SafeCorpusPath",
    "Get-PngDimensions",
    "Get-PdfPageCount",
    "Invoke-PdfRaster",
    "Get-WordSemanticInventory",
    "Get-ExcelSemanticInventory",
    "Get-PowerPointSemanticInventory"
)
