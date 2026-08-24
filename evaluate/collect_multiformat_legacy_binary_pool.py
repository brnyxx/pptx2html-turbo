from __future__ import annotations

import argparse
from collections.abc import Sequence
from pathlib import Path

from evaluate.collect_multiformat_public_pool import _fetch_blob, _fetch_tree
from evaluate.multiformat_legacy_binary_pool import (
    LegacyBinaryPoolError,
    collect_legacy_binary_pool,
)


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(
        description="Collect pinned non-blind DOC, XLS, and PPT conformance sources.",
    )
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--public-config", type=Path, required=True)
    parser.add_argument("--blind-manifest", type=Path, required=True)
    parser.add_argument("--output-dir", type=Path, required=True)
    arguments = parser.parse_args(argv)
    try:
        collect_legacy_binary_pool(
            arguments.config,
            arguments.public_config,
            arguments.blind_manifest,
            arguments.output_dir,
            tree_fetcher=_fetch_tree,
            blob_fetcher=_fetch_blob,
        )
    except LegacyBinaryPoolError as error:
        parser.error(str(error))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
