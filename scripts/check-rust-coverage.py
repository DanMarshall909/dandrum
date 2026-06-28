#!/usr/bin/env python3
"""Check strict Rust line coverage against an explicit uncovered-line allowlist."""

from __future__ import annotations

import subprocess
import sys
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
MANIFEST = ROOT / "src" / "rust-engine" / "Cargo.toml"
POLICY = ROOT / "coverage-allowlist.txt"


def main() -> int:
    strict_files, allowed = read_policy(POLICY)
    if not strict_files:
        print(f"coverage policy has no strict files: {POLICY}", file=sys.stderr)
        return 2

    result = subprocess.run(
        [
            str(Path.home() / ".cargo" / "bin" / "cargo"),
            "llvm-cov",
            "--manifest-path",
            str(MANIFEST),
            "--show-missing-lines",
            "--summary-only",
        ],
        cwd=ROOT,
        stdout=subprocess.PIPE,
        stderr=sys.stderr,
        text=True,
        check=False,
    )
    if result.returncode != 0:
        return result.returncode

    uncovered = parse_missing_lines(result.stdout)
    failures: list[str] = []
    stale_allowlist: list[str] = []

    for path in sorted(strict_files):
        file_uncovered = uncovered.get(path, set())
        file_allowed = allowed_for(path, allowed)
        unexpected = sorted(file_uncovered - file_allowed)
        if unexpected:
            failures.append(f"{path}: uncovered lines {format_lines(unexpected)}")

        stale = sorted(file_allowed - file_uncovered)
        for line in stale:
            stale_allowlist.append(f"{path}:{line}")

    if failures or stale_allowlist:
        if failures:
            print("Coverage check failed; add tests or justify lines in coverage-allowlist.txt.")
            for failure in failures:
                print(f"- {failure}")
        if stale_allowlist:
            print("Stale coverage allowlist entries should be removed:")
            for entry in stale_allowlist:
                print(f"- {entry}")
        return 1

    print("Coverage check passed for strict files.")
    return 0


def read_policy(path: Path) -> tuple[set[str], dict[tuple[str, int], str]]:
    strict_files: set[str] = set()
    allowed: dict[tuple[str, int], str] = {}

    for number, raw_line in enumerate(path.read_text().splitlines(), start=1):
        line = raw_line.strip()
        if not line or line.startswith("#"):
            continue

        parts = line.split(maxsplit=2)
        directive = parts[0]
        if directive == "strict" and len(parts) == 2:
            strict_files.add(parts[1])
        elif directive == "allow" and len(parts) == 3:
            location, reason = parts[1], parts[2]
            file_path, line_number = parse_location(location, number)
            if not reason.strip():
                raise PolicyError(f"line {number}: allow entry requires a reason")
            allowed[(file_path, line_number)] = reason
        else:
            raise PolicyError(
                f"line {number}: expected 'strict <path>' or 'allow <path>:<line> <reason>'"
            )

    return strict_files, allowed


def parse_location(location: str, policy_line: int) -> tuple[str, int]:
    file_path, separator, line_text = location.rpartition(":")
    if not separator or not file_path or not line_text.isdigit():
        raise PolicyError(f"line {policy_line}: invalid location {location!r}")
    return file_path, int(line_text)


def parse_missing_lines(output: str) -> dict[str, set[int]]:
    marker = "Uncovered Lines:"
    if marker not in output:
        return {}

    files: dict[str, set[int]] = {}
    for raw_line in output.split(marker, maxsplit=1)[1].splitlines():
        line = raw_line.strip()
        if not line:
            continue
        location, separator, numbers = line.partition(": ")
        if not separator:
            continue
        absolute = Path(location)
        try:
            relative = absolute.relative_to(ROOT).as_posix()
        except ValueError:
            continue
        files[relative] = {
            int(number.strip())
            for number in numbers.split(",")
            if number.strip().isdigit()
        }
    return files


def allowed_for(path: str, allowed: dict[tuple[str, int], str]) -> set[int]:
    return {line for (file_path, line), _reason in allowed.items() if file_path == path}


def format_lines(lines: list[int]) -> str:
    if len(lines) <= 20:
        return ", ".join(str(line) for line in lines)
    shown = ", ".join(str(line) for line in lines[:20])
    return f"{shown}, ... ({len(lines)} total)"


class PolicyError(Exception):
    pass


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except PolicyError as error:
        print(f"coverage policy error: {error}", file=sys.stderr)
        raise SystemExit(2)
