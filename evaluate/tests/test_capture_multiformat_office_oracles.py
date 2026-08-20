import shutil
import subprocess
import unittest
from pathlib import Path

PROJECT_ROOT = Path(__file__).resolve().parents[2]
SCRIPT_PATH = PROJECT_ROOT / "evaluate" / "capture_multiformat_office_oracles.ps1"
MODULE_PATH = PROJECT_ROOT / "evaluate" / "multiformat" / "OfficeOracle.psm1"


class CaptureMultiFormatOfficeOraclesTests(unittest.TestCase):
    def test_script_uses_native_office_export_methods_and_macro_lockdown(self) -> None:
        # Given
        script = SCRIPT_PATH.read_text(encoding="utf-8")
        module = MODULE_PATH.read_text(encoding="utf-8")
        combined = script + module

        # When and Then
        self.assertIn("Word.Application", script)
        self.assertIn("Excel.Application", script)
        self.assertIn("PowerPoint.Application", script)
        self.assertGreaterEqual(combined.count("AutomationSecurity = 3"), 3)
        self.assertIn("ExportAsFixedFormat($pdfPath, 17)", script)
        self.assertIn("ExportAsFixedFormat(0, $pdfPath)", script)
        self.assertIn('.Export($slidePath, "PNG", 960, 540)', script)
        self.assertIn("ExportAsFixedFormat($pdfPath, 2)", script)
        self.assertIn("network_isolation = $HostNetworkIsolation", script)

    @unittest.skipUnless(shutil.which("pwsh"), "PowerShell is not installed")
    def test_powershell_files_parse_without_syntax_errors(self) -> None:
        for path in [SCRIPT_PATH, MODULE_PATH]:
            # Given
            command = (
                "$tokens=$null;$errors=$null;"
                f"[System.Management.Automation.Language.Parser]::ParseFile('{path}',"
                "[ref]$tokens,[ref]$errors)>$null;"
                "if($errors.Count -ne 0){$errors|ForEach-Object{Write-Error $_};exit 1}"
            )

            # When
            result = subprocess.run(
                ["pwsh", "-NoProfile", "-Command", command],
                check=False,
                capture_output=True,
                text=True,
            )

            # Then
            self.assertEqual(result.returncode, 0, result.stderr)
