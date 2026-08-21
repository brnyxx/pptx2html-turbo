from __future__ import annotations

import argparse
from pathlib import Path

from evaluate.multiformat_public_pool import (
    PublicPoolError,
    validate_public_pool,
)


def main() -> None:
    parser = argparse.ArgumentParser(
        description="Validate an exact pinned public blind source pool.",
    )
    parser.add_argument("--config", type=Path, required=True)
    parser.add_argument("--manifest", type=Path, required=True)
    arguments = parser.parse_args()
    try:
        validate_public_pool(arguments.config, arguments.manifest)
    except PublicPoolError as error:
        parser.error(str(error))


if __name__ == "__main__":
    main()
