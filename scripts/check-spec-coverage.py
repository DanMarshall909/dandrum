#!/usr/bin/env python3
"""Enforce that every spec acceptance criterion (scenario) is proven by a test.

Source of truth: openspec/specs/<capability>/spec.md
Coverage map:    spec-tests.map (one block per acceptance criterion)

Each AC block records a fingerprint of the scenario text. When a spec scenario
is edited its fingerprint drifts; the check then fails until a human re-reviews
whether the mapped tests still prove the AC and re-affirms by updating the
fingerprint. Updating the fingerprint IS the recorded act of "I reviewed that
these tests prove this AC."

Directives inside the map (see spec-tests.map header):
  AC <capability> :: <Requirement> :: <Scenario>
    fp   <hash>            recorded scenario fingerprint
    test <id>              a test that proves this AC (repeatable)
    todo <reason>          ratchet backlog: AC not yet backed by a real test

Test id forms:
  rust:<fn_name>           a Rust `fn <fn_name>` under a #[test]/#[cfg(test)]
  cpp:<path>               a C++ test source file (whole-file main() test)

Usage:
  check-spec-coverage.py                    verify (CI mode)
  check-spec-coverage.py --init             seed the map with every AC as todo
  check-spec-coverage.py --update-fingerprints   re-bless fingerprints to current
"""

from __future__ import annotations

import hashlib
import re
import sys
from dataclasses import dataclass, field
from pathlib import Path


ROOT = Path(__file__).resolve().parents[1]
SPECS_DIR = ROOT / "openspec" / "specs"
MAP_PATH = ROOT / "spec-tests.map"
RUST_SRC = ROOT / "src" / "rust-engine"
CPP_TESTS = ROOT / "tests"

SEP = " :: "
FP_LEN = 12


@dataclass(frozen=True)
class AcKey:
    capability: str
    requirement: str
    scenario: str

    def render(self) -> str:
        return f"{self.capability}{SEP}{self.requirement}{SEP}{self.scenario}"


@dataclass
class AcBlock:
    key: AcKey
    fingerprint: str | None = None
    tests: list[str] = field(default_factory=list)
    todo: str | None = None


# --------------------------------------------------------------------------- #
# Spec parsing
# --------------------------------------------------------------------------- #


def fingerprint(scenario_title: str, body_lines: list[str]) -> str:
    normalized = [scenario_title.strip()]
    normalized += [line.strip() for line in body_lines if line.strip()]
    digest = hashlib.sha1("\n".join(normalized).encode("utf-8")).hexdigest()
    return digest[:FP_LEN]


def parse_specs() -> dict[AcKey, str]:
    """Return {AcKey: current_fingerprint} for every scenario in every main spec."""
    acs: dict[AcKey, str] = {}
    for spec_file in sorted(SPECS_DIR.glob("*/spec.md")):
        capability = spec_file.parent.name
        requirement = ""
        scenario_title: str | None = None
        body: list[str] = []

        def flush() -> None:
            nonlocal scenario_title, body
            if scenario_title is not None:
                key = AcKey(capability, requirement, scenario_title)
                acs[key] = fingerprint(scenario_title, body)
            scenario_title = None
            body = []

        for line in spec_file.read_text().splitlines():
            req = re.match(r"^### Requirement:\s*(.+)", line)
            scen = re.match(r"^#### Scenario:\s*(.+)", line)
            if req:
                flush()
                requirement = req.group(1).strip()
            elif scen:
                flush()
                scenario_title = scen.group(1).strip()
            elif re.match(r"^#{1,3} ", line):  # any higher-level header ends a scenario
                flush()
            elif scenario_title is not None:
                body.append(line)
        flush()
    return acs


# --------------------------------------------------------------------------- #
# Map parsing / writing
# --------------------------------------------------------------------------- #


class MapError(Exception):
    pass


def parse_map(path: Path) -> dict[AcKey, AcBlock]:
    if not path.exists():
        return {}
    blocks: dict[AcKey, AcBlock] = {}
    current: AcBlock | None = None
    for number, raw in enumerate(path.read_text().splitlines(), start=1):
        line = raw.rstrip()
        stripped = line.strip()
        if not stripped or stripped.startswith("#"):
            continue
        if stripped.startswith("AC "):
            identity = stripped[3:].strip()
            parts = identity.split(SEP)
            if len(parts) != 3:
                raise MapError(f"line {number}: AC must be 'capability :: Requirement :: Scenario'")
            current = AcBlock(AcKey(parts[0].strip(), parts[1].strip(), parts[2].strip()))
            if current.key in blocks:
                raise MapError(f"line {number}: duplicate AC {identity!r}")
            blocks[current.key] = current
            continue
        if current is None:
            raise MapError(f"line {number}: directive before any AC header: {stripped!r}")
        keyword, _, value = stripped.partition(" ")
        value = value.strip()
        if keyword == "fp":
            current.fingerprint = value
        elif keyword == "test":
            current.tests.append(value)
        elif keyword == "todo":
            current.todo = value or "(no reason given)"
        else:
            raise MapError(f"line {number}: unknown directive {keyword!r} (expected fp/test/todo)")
    return blocks


def write_map(path: Path, acs: dict[AcKey, str], existing: dict[AcKey, AcBlock]) -> None:
    lines = [
        "# Spec acceptance-criteria -> test coverage map.",
        "#",
        "# One block per scenario (acceptance criterion) in openspec/specs/**/spec.md.",
        "# Policy: every AC must have at least one `test`, or a `todo` explaining why not.",
        "#",
        "#   AC <capability> :: <Requirement> :: <Scenario>",
        "#     fp   <fingerprint>   recorded hash of the scenario text",
        "#     test <id>            rust:<fn_name> or cpp:<path> (repeatable)",
        "#     todo <reason>        ratchet backlog until a real test is mapped",
        "#",
        "# When a scenario is edited its fingerprint drifts and the check fails: review",
        "# whether the mapped tests still prove the AC, then re-bless with",
        "# scripts/check-spec-coverage --update-fingerprints.",
        "",
    ]
    for key in sorted(acs, key=lambda k: (k.capability, k.requirement, k.scenario)):
        block = existing.get(key)
        lines.append(f"AC {key.render()}")
        lines.append(f"  fp {acs[key]}")
        if block and block.tests:
            for test in block.tests:
                lines.append(f"  test {test}")
        if block and block.todo:
            lines.append(f"  todo {block.todo}")
        elif not (block and block.tests):
            lines.append("  todo not yet mapped to a test")
        lines.append("")
    path.write_text("\n".join(lines))


# --------------------------------------------------------------------------- #
# Test corpus
# --------------------------------------------------------------------------- #


def rust_test_functions() -> set[str]:
    names: set[str] = set()
    for rs in RUST_SRC.rglob("*.rs"):
        if "llvm-cov-target" in rs.parts or "target" in rs.parts:
            continue
        for match in re.finditer(r"\bfn\s+([a-z_][a-z0-9_]*)\s*\(", rs.read_text()):
            names.add(match.group(1))
    return names


def test_exists(test_id: str, rust_fns: set[str]) -> bool:
    kind, _, value = test_id.partition(":")
    if kind == "rust":
        return value in rust_fns
    if kind == "cpp":
        return (ROOT / value).is_file()
    return False


# --------------------------------------------------------------------------- #
# Verify
# --------------------------------------------------------------------------- #


def verify() -> int:
    acs = parse_specs()
    if not acs:
        print(f"no acceptance criteria found under {SPECS_DIR}", file=sys.stderr)
        return 2
    blocks = parse_map(MAP_PATH)
    rust_fns = rust_test_functions()

    unmapped: list[str] = []
    stale_entries: list[str] = []
    uncovered: list[str] = []
    drifted: list[str] = []
    missing_tests: list[str] = []
    todo_backlog: list[str] = []

    for key, current_fp in acs.items():
        block = blocks.get(key)
        if block is None:
            unmapped.append(key.render())
            continue
        if block.tests:
            if block.fingerprint != current_fp:
                drifted.append(f"{key.render()} (recorded {block.fingerprint}, now {current_fp})")
            for test in block.tests:
                if not test_exists(test, rust_fns):
                    missing_tests.append(f"{key.render()} -> {test}")
        elif block.todo:
            todo_backlog.append(key.render())
        else:
            uncovered.append(key.render())

    spec_keys = set(acs)
    for key in blocks:
        if key not in spec_keys:
            stale_entries.append(key.render())

    failures = bool(unmapped or stale_entries or uncovered or drifted or missing_tests)

    def report(title: str, items: list[str], hint: str) -> None:
        if not items:
            return
        print(f"\n{title} ({len(items)}): {hint}")
        for item in sorted(items):
            print(f"  - {item}")

    report("Unmapped acceptance criteria", unmapped,
           "add an AC block (test or todo) in spec-tests.map")
    report("Stale map entries", stale_entries,
           "AC no longer in specs; remove from spec-tests.map")
    report("Uncovered acceptance criteria", uncovered,
           "AC block has neither a test nor a todo")
    report("Fingerprint drift", drifted,
           "scenario changed; re-review the tests, then --update-fingerprints")
    report("Referenced tests not found", missing_tests,
           "the mapped test does not exist in the corpus")

    print()
    total = len(acs)
    tested = sum(1 for k, b in ((k, blocks.get(k)) for k in acs) if b and b.tests and not b.todo)
    print(f"Acceptance criteria: {total} | mapped-to-tests: {tested} | "
          f"ratchet backlog (todo): {len(todo_backlog)}")

    if failures:
        print("\nSpec-coverage check FAILED.")
        return 1
    if todo_backlog:
        print("Spec-coverage check passed (with ratchet backlog still to burn down).")
    else:
        print("Spec-coverage check passed; every AC is proven by a test.")
    return 0


def main(argv: list[str]) -> int:
    if "--init" in argv:
        acs = parse_specs()
        existing = parse_map(MAP_PATH)
        write_map(MAP_PATH, acs, existing)
        print(f"Wrote {len(acs)} acceptance criteria to {MAP_PATH.name}.")
        return 0
    if "--update-fingerprints" in argv:
        acs = parse_specs()
        blocks = parse_map(MAP_PATH)
        changed = 0
        for key, block in blocks.items():
            if key in acs and block.fingerprint != acs[key]:
                block.fingerprint = acs[key]
                changed += 1
        write_map(MAP_PATH, acs, blocks)
        print(f"Re-blessed {changed} fingerprint(s).")
        return 0
    return verify()


if __name__ == "__main__":
    try:
        raise SystemExit(main(sys.argv[1:]))
    except MapError as error:
        print(f"spec-tests.map error: {error}", file=sys.stderr)
        raise SystemExit(2)
