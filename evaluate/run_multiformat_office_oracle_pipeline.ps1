param(
    [Parameter(Mandatory = $true)]
    [string]$InputBundleDir,

    [Parameter(Mandatory = $true)]
    [string]$OutputDir,

    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-f]{40}$")]
    [string]$ProjectRevision,

    [Parameter(Mandatory = $true)]
    [ValidatePattern("^[0-9a-f]{64}$")]
    [string]$RunNonce,

    [string]$PythonPath = "python",
    [string]$ReceiptSignerPath = $env:OFFICE_ORACLE_RECEIPT_SIGNER,
    [string]$PublicKeyPath = $env:OFFICE_ORACLE_PUBLIC_KEY,
    [string]$OpenSslPath = $env:OFFICE_ORACLE_OPENSSL,
    [string]$PdfInfoPath = "pdfinfo",
    [string]$PdfToPpmPath = "pdftoppm",
    [string]$PdfToTextPath = "pdftotext"
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"

function Get-ScopedNonce {
    param(
        [Parameter(Mandatory = $true)][string]$Nonce,
        [Parameter(Mandatory = $true)][string]$Scope
    )

    $bytes = [Text.Encoding]::UTF8.GetBytes("$Nonce`:$Scope")
    return [Convert]::ToHexString(
        [Security.Cryptography.SHA256]::HashData($bytes)
    ).ToLowerInvariant()
}

foreach ($path in @($ReceiptSignerPath, $PublicKeyPath, $OpenSslPath)) {
    if ([string]::IsNullOrWhiteSpace($path) -or -not (Test-Path -LiteralPath $path -PathType Leaf)) {
        throw "Trusted Office oracle runtime path is missing"
    }
}

$bundle = (Resolve-Path -LiteralPath $InputBundleDir).Path
if (Test-Path -LiteralPath $OutputDir) {
    if (@(Get-ChildItem -LiteralPath $OutputDir -Force).Count -ne 0) {
        throw "Office oracle output directory must be empty"
    }
}
else {
    New-Item -ItemType Directory -Path $OutputDir | Out-Null
}
$output = (Resolve-Path -LiteralPath $OutputDir).Path
$contractPath = Join-Path $bundle "contract.json"
$oracleLockPath = Join-Path $bundle "oracle-lock.json"
$evaluatorPath = Join-Path $bundle "evaluator-manifest.json"
$inputManifestPath = Join-Path $bundle "office-input-manifest.json"
$contract = Get-Content -LiteralPath $contractPath -Raw | ConvertFrom-Json
$oracleLock = Get-Content -LiteralPath $oracleLockPath -Raw | ConvertFrom-Json
$rawOutput = Join-Path $output "raw"
$finalOutput = Join-Path $output "final"
New-Item -ItemType Directory -Path $finalOutput | Out-Null

& (Join-Path $PSScriptRoot "capture_multiformat_office_oracles.ps1") `
    -InputManifest $inputManifestPath `
    -OutputDir $rawOutput `
    -GoldenSetRevision $ProjectRevision `
    -FontBundleSha256 ([string]$oracleLock.font_bundle_sha256) `
    -HostNetworkIsolation disabled `
    -WindowsVersion ([Environment]::OSVersion.VersionString) `
    -OfficeChannel ([string]$oracleLock.office.channel) `
    -PdfInfoPath $PdfInfoPath `
    -PdfToPpmPath $PdfToPpmPath `
    -PdfToTextPath $PdfToTextPath
if ($LASTEXITCODE -ne 0) {
    throw "Office oracle capture failed"
}

foreach ($format in @($contract.required_formats)) {
    $corpusPath = Join-Path $bundle "corpora/$format/manifest.json"
    $formatOutput = Join-Path $finalOutput $format
    $scopedNonce = Get-ScopedNonce -Nonce $RunNonce -Scope $format
    & $PythonPath -m evaluate.finalize_multiformat_office_oracles `
        --batch-manifest (Join-Path $rawOutput "manifest.json") `
        --contract $contractPath `
        --corpus-manifest $corpusPath `
        --evaluator-manifest $evaluatorPath `
        --oracle-lock $oracleLockPath `
        --output-dir $formatOutput `
        --receipt-signer $ReceiptSignerPath `
        --public-key $PublicKeyPath `
        --openssl $OpenSslPath `
        --project-revision $ProjectRevision `
        --run-nonce $scopedNonce
    if ($LASTEXITCODE -ne 0) {
        throw "Office oracle finalization failed for $format"
    }
}
