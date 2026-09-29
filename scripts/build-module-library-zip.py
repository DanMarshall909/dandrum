#!/usr/bin/env python3
"""Build or check the deterministic, stored standard-module-library seed zip."""

from __future__ import annotations

import io
import sys
import zipfile
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SOURCE = ROOT / "src/rust-engine/module-library/1.0.0"
OUTPUT = ROOT / "src/rust-engine/module-library/seed-1.0.0.zip"


def build() -> bytes:
    buffer = io.BytesIO()
    with zipfile.ZipFile(buffer, "w") as archive:
        for path in sorted(item for item in SOURCE.rglob("*") if item.is_file()):
            name = path.relative_to(SOURCE).as_posix()
            info = zipfile.ZipInfo(name, (1980, 1, 1, 0, 0, 0))
            info.compress_type = zipfile.ZIP_STORED
            info.create_system = 3
            info.external_attr = 0o100644 << 16
            archive.writestr(info, path.read_bytes())
    return buffer.getvalue()


def main() -> int:
    expected = build()
    if len(sys.argv) == 2 and sys.argv[1] == "--check":
        if not OUTPUT.exists() or OUTPUT.read_bytes() != expected:
            print(f"stale module-library seed zip: {OUTPUT}", file=sys.stderr)
            return 1
        print(f"module-library seed zip is current: {OUTPUT}")
        return 0
    if len(sys.argv) != 1:
        print("usage: build-module-library-zip.py [--check]", file=sys.stderr)
        return 2
    OUTPUT.write_bytes(expected)
    print(f"wrote {OUTPUT}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
