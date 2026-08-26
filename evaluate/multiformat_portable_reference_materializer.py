from __future__ import annotations

from pathlib import Path

from evaluate.multiformat_portable_receipt_trust import (
    load_portable_receipt_trust,
    verify_trusted_files,
)
from evaluate.multiformat_portable_reference_artifacts import load_raw_private_key
from evaluate.multiformat_portable_reference_manifest import (
    write_portable_reference_manifests,
)
from evaluate.multiformat_portable_reference_runner import (
    PortableReferenceTools,
    run_reference_source,
)
from evaluate.multiformat_portable_reference_sources import (
    bind_trusted_sources,
    load_reference_sources,
)
from evaluate.multiformat_reference_routing import (
    DocumentFormat as RoutingDocumentFormat,
)
from evaluate.multiformat_reference_routing import (
    load_reference_routing,
)
from evaluate.multiformat_schema import object_value, string_value
from evaluate.multiformat_strict_json import read_strict_object

ROUTING = Path(__file__).parent / "multiformat/reference-routing.v1.json"


class PortableReferenceMaterializeError(ValueError):
    pass


def materialize_portable_references(
    contract: Path,
    corpus_manifest: Path,
    portable_lock: Path,
    evidence_root: Path,
    output_dir: Path,
    private_key: Path,
    *,
    nonce: str,
    batch_id: str,
) -> Path:
    try:
        root = evidence_root.resolve(strict=True)
        destination = output_dir.parent.resolve(strict=True) / output_dir.name
        if not destination.is_relative_to(root) or destination.exists():
            raise PortableReferenceMaterializeError(
                "portable output must be new inside evidence root"
            )
        trust = load_portable_receipt_trust(portable_lock, root)
        sources = load_reference_sources(contract, corpus_manifest)
        trusted = tuple((source.sha256, root / source.path) for source in trust.sources)
        sources = bind_trusted_sources(sources, trusted)
        by_role = {item.role: root / item.path for item in trust.lock_artifacts}
        lock = read_strict_object(portable_lock)
        sandbox = object_value(lock, "sandbox")

        def verify_runtime() -> None:
            _ = verify_trusted_files(trust)

        tools = PortableReferenceTools(
            by_role["tool:libreoffice"],
            by_role["tool:poppler-metadata"],
            by_role["tool:poppler-render"],
            by_role["tool:poppler-text"],
            root / string_value(object_value(sandbox, "executable"), "path"),
            root / string_value(object_value(sandbox, "profile"), "path"),
            verify_runtime,
        )
        routing = load_reference_routing(ROUTING)
        destination.mkdir()
        batches = [
            run_reference_source(
                source,
                RoutingDocumentFormat(sources.document_format.value),
                routing,
                tools,
                destination / "raw" / source.source_id,
            )
            for source in sources.sources
        ]
        return write_portable_reference_manifests(
            destination,
            sources,
            batches,
            trust,
            load_raw_private_key(private_key),
            nonce=nonce,
            batch_id=batch_id,
        )
    except PortableReferenceMaterializeError:
        raise
    except (KeyError, OSError, TypeError, ValueError) as error:
        raise PortableReferenceMaterializeError(
            "portable reference materialization failed"
        ) from error
