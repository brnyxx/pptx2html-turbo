from __future__ import annotations

import json
import tempfile
import unittest
from pathlib import Path

from evaluate.multiformat_portable_lock import (
    PortableLockError,
    validate_reference_lock,
)
from evaluate.multiformat_portable_package_inventory import (
    bind_package_executable_with_inventory,
    package_binding,
)
from evaluate.multiformat_schema import JsonValue, sha256_file
from evaluate.tests import test_multiformat_portable_lock as portable_lock_test


class PortableNativeRuntimeContractTests(unittest.TestCase):
    def test_native_outer_inventories_require_matching_schema_two_inner_lock(
        self,
    ) -> None:
        mutations = (
            "schema-one",
            "missing-poppler",
            "missing-openssl",
            "mismatched-poppler",
            "mismatched-openssl",
        )
        for mutation in mutations:
            with self.subTest(mutation=mutation), tempfile.TemporaryDirectory() as temp:
                # Given: both outer native packages and a matching schema-2 inner lock.
                root, lock_path, lock = self._native_lock(Path(temp))
                runtime = json.loads(
                    self._runtime_path(root, lock).read_text(encoding="utf-8")
                )
                if mutation == "schema-one":
                    runtime["schema_version"] = 1
                elif mutation == "missing-poppler":
                    self._mapping(runtime, "candidate_runtime").pop(
                        "poppler_package_inventory_sha256"
                    )
                elif mutation == "missing-openssl":
                    self._mapping(runtime, "sandbox_verifier").pop(
                        "openssl_package_inventory_sha256"
                    )
                elif mutation == "mismatched-poppler":
                    self._mapping(runtime, "candidate_runtime")[
                        "poppler_package_inventory_sha256"
                    ] = "0" * 64
                else:
                    self._mapping(runtime, "sandbox_verifier")[
                        "openssl_package_inventory_sha256"
                    ] = "1" * 64
                self._write_bound_runtime(root, lock, runtime)

                # When/Then: outer validation rejects the unbound inner identity.
                with self.assertRaises(PortableLockError):
                    validate_reference_lock(lock_path, root)

    def test_matching_native_outer_and_schema_two_inner_lock_is_valid(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            # Given: both outer package inventories bound into the schema-2 inner lock.
            root, lock_path, _lock = self._native_lock(Path(temp))

            # When: the complete outer lock is validated.
            identity = validate_reference_lock(lock_path, root)

            # Then: the portable native profile remains ready.
            self.assertEqual(identity.schema_version, 2)

    def test_schema_one_inner_lock_remains_valid_for_linux_flat_tools(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            # Given: a Linux flat-tool outer lock and its legacy schema-1 inner lock.
            root, lock_path, lock = (
                portable_lock_test.MultiFormatPortableLockTests._portable_lock(
                    Path(temp)
                )
            )
            self._set_platform(root, lock, ("Linux", "x86_64"))
            self._write_bound_runtime(root, lock, {"schema_version": 1})

            # When: the flat portable lock is validated.
            identity = validate_reference_lock(lock_path, root)

            # Then: schema-1 inner compatibility is preserved.
            self.assertEqual(identity.schema_version, 2)

    def test_linux_flat_inner_lock_tampering_breaks_outer_binding(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            # Given: a valid Linux flat-tool lock with a bound schema-1 inner lock.
            root, lock_path, lock = (
                portable_lock_test.MultiFormatPortableLockTests._portable_lock(
                    Path(temp)
                )
            )
            self._set_platform(root, lock, ("Linux", "x86_64"))
            self._write_bound_runtime(root, lock, {"schema_version": 1})

            # When: the inner runtime bytes change without rebinding the outer lock.
            self._runtime_path(root, lock).write_text(
                json.dumps({"schema_version": 1, "tampered": True}),
                encoding="utf-8",
            )

            # Then: outer validation rejects the stale candidate-runtime digest.
            with self.assertRaises(PortableLockError):
                validate_reference_lock(lock_path, root)

    def test_darwin_outer_lock_requires_all_native_inventories(self) -> None:
        with tempfile.TemporaryDirectory() as temp:
            # Given: an inventory-less Darwin outer lock.
            root, lock_path, lock = (
                portable_lock_test.MultiFormatPortableLockTests._portable_lock(
                    Path(temp)
                )
            )
            self._set_platform(root, lock, ("Darwin", "arm64"))
            self._write_bound_runtime(root, lock, {"schema_version": 1})

            # When/Then: Darwin cannot claim a flat native runtime.
            with self.assertRaises(PortableLockError):
                validate_reference_lock(lock_path, root)

    def test_one_sided_outer_native_inventory_is_rejected(self) -> None:
        for side in ("poppler", "openssl"):
            with self.subTest(side=side), tempfile.TemporaryDirectory() as temp:
                # Given: only one native package inventory is present in the outer lock.
                root, lock_path, lock = (
                    portable_lock_test.MultiFormatPortableLockTests._portable_lock(
                        Path(temp)
                    )
                )
                self._bind_outer_package(root, lock, side)
                self._write_bound_runtime(root, lock, {"schema_version": 1})

                # When/Then: the incomplete native profile is rejected.
                with self.assertRaises(PortableLockError):
                    validate_reference_lock(lock_path, root)

    @classmethod
    def _native_lock(cls, temp: Path) -> tuple[Path, Path, dict[str, JsonValue]]:
        root, lock_path, lock = (
            portable_lock_test.MultiFormatPortableLockTests._portable_lock(temp)
        )
        cls._set_platform(root, lock, ("Darwin", "arm64"))
        poppler_sha = cls._bind_outer_package(root, lock, "poppler")
        openssl_sha = cls._bind_outer_package(root, lock, "openssl")
        cls._write_bound_runtime(
            root,
            lock,
            {
                "schema_version": 2,
                "candidate_runtime": {"poppler_package_inventory_sha256": poppler_sha},
                "sandbox_verifier": {"openssl_package_inventory_sha256": openssl_sha},
            },
        )
        return root, lock_path, lock

    @classmethod
    def _bind_outer_package(
        cls, root: Path, lock: dict[str, JsonValue], package_name: str
    ) -> str:
        source = root.parent / f"{root.name}-{package_name}.app"
        if package_name == "poppler":
            executable = source / "Contents/bin/pdftoppm"
            executable.parent.mkdir(parents=True)
            for name in ("pdftoppm", "pdftotext", "pdfinfo"):
                executable.with_name(name).write_bytes(name.encode())
            library = source / "Contents/lib/libpoppler.dylib"
            library.parent.mkdir(parents=True)
            library.write_bytes(b"library")
            bound, inventory = bind_package_executable_with_inventory(
                executable, root, root / "artifacts/poppler-package"
            )
            if inventory is None:
                raise AssertionError("test Poppler package inventory is missing")
            tools = cls._mapping(lock, "tools")
            for field, name in (
                ("poppler_render", "pdftoppm"),
                ("poppler_text", "pdftotext"),
                ("poppler_metadata", "pdfinfo"),
            ):
                tools[field] = package_binding(
                    root, bound.with_name(name), "test", inventory
                )
            return sha256_file(inventory)
        executable = source / "Contents/bin/openssl"
        executable.parent.mkdir(parents=True)
        executable.write_bytes(b"openssl")
        bound, inventory = bind_package_executable_with_inventory(
            executable, root, root / "artifacts/openssl-package"
        )
        if inventory is None:
            raise AssertionError("test OpenSSL package inventory is missing")
        cls._mapping(lock, "candidate_sandbox")["openssl"] = package_binding(
            root, bound, "test", inventory
        )
        return sha256_file(inventory)

    @classmethod
    def _set_platform(
        cls,
        root: Path,
        lock: dict[str, JsonValue],
        identity: tuple[str, str],
    ) -> None:
        system, architecture = identity
        platform = cls._mapping(lock, "platform")
        platform["os"] = system
        platform["architecture"] = architecture
        runtime = cls._mapping(lock, "runtime")
        attestation_binding = cls._mapping(runtime, "attestation")
        attestation_path = root / cls._string(attestation_binding, "path")
        attestation = json.loads(attestation_path.read_text(encoding="utf-8"))
        attestation["os"] = system
        attestation["architecture"] = architecture
        attestation_path.write_text(
            json.dumps(attestation, sort_keys=True), encoding="utf-8"
        )
        attestation_binding["sha256"] = sha256_file(attestation_path)

    @classmethod
    def _write_bound_runtime(
        cls,
        root: Path,
        lock: dict[str, JsonValue],
        runtime: dict[str, JsonValue],
    ) -> None:
        runtime_path = cls._runtime_path(root, lock)
        runtime_path.write_text(json.dumps(runtime, sort_keys=True), encoding="utf-8")
        cls._mapping(lock, "candidate_runtime_lock")["sha256"] = sha256_file(
            runtime_path
        )
        portable_lock_test.MultiFormatPortableLockTests._write(
            root / "oracle-lock.json", lock
        )

    @classmethod
    def _runtime_path(cls, root: Path, lock: dict[str, JsonValue]) -> Path:
        return root / cls._string(cls._mapping(lock, "candidate_runtime_lock"), "path")

    @staticmethod
    def _mapping(values: dict[str, JsonValue], field: str) -> dict[str, JsonValue]:
        return portable_lock_test.MultiFormatPortableLockTests._mapping(values, field)

    @staticmethod
    def _string(values: dict[str, JsonValue], field: str) -> str:
        return portable_lock_test.MultiFormatPortableLockTests._string(values, field)


if __name__ == "__main__":
    unittest.main()
