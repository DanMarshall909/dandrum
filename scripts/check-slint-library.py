#!/usr/bin/env python3
"""Compile the DanDrum Slint library and exercise its live interaction fixtures."""
from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import shutil
import subprocess
import sys

from slint_headless import prepare_headless, use_environment

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "tests/slint"))
from mcp_client import ViewerSession
from live_checks import LIVE_CHECKS
from catalog_checks import catalog as check_catalog

ROOT_TYPES = {"Controls": "ControlsTest", "Displays": "Displays", "Layout": "LayoutTests",
              "Menus": "MenuTests", "Waveform": "WaveformTests", "DisplayEdges": "DisplayEdges",
              "LayerDetails": "LayerDetailsTests", "Readouts": "ReadoutTests", "DisplayBindings": "DisplayBindings"}
REPORT_IDS = {"Layout": "test-report", "Menus": "test-report"}


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--viewer", default=os.environ.get("DANDRUM_SLINT_VIEWER", "slint-viewer"))
    parser.add_argument("--check-only", action="store_true", help="Compile without opening a window or requiring a display")
    parser.add_argument("--headless", action="store_true", help="Run the same live input/screenshot checks on a private Linux Xvfb display")
    scope = parser.add_mutually_exclusive_group()
    scope.add_argument("--fixture", action="append", choices=sorted(ROOT_TYPES), help="Limit checks to one fixture; may repeat")
    scope.add_argument("--catalog-only", action="store_true", help="Check only the composed catalog")
    parser.add_argument("--evidence", type=Path, default=ROOT / "build/slint-library-evidence")
    args = parser.parse_args()
    try:
        plan = prepare_headless(sys.argv[1:], os.environ, script=Path(__file__).resolve())
        if plan is not None and plan.command is not None:
            return subprocess.run(plan.command, env=plan.environment).returncode
        with use_environment(plan):
            return run_checks(args, parser)
    except (OSError, RuntimeError) as error:
        parser.error(str(error))


def run_checks(args, parser):
    viewer = shutil.which(args.viewer)
    if not viewer:
        parser.error("slint-viewer was not found; set DANDRUM_SLINT_VIEWER or pass --viewer")
    args.evidence.mkdir(parents=True, exist_ok=True)
    fixtures = [] if args.catalog_only else [ROOT / f"tests/slint/{name}.slint" for name in (args.fixture or ROOT_TYPES)]
    sources = list(fixtures)
    catalog = ROOT / "ui/slint/catalog/Catalog.slint"
    if not args.fixture:
        sources.append(catalog)
    results = {"viewer": subprocess.check_output([viewer, "--version"], text=True).strip(),
               "backend": None if args.check_only else os.environ.get("SLINT_BACKEND", "winit-software"),
               "platform": sys.platform,
               "mode": "compile-only" if args.check_only else "live-interactions",
               "headless": not args.check_only and os.environ.get("DANDRUM_SLINT_HEADLESS") == "1",
               "compilation": [], "contracts": [], "interactions": []}
    try:
        for source in sources:
            subprocess.run([viewer, "--check", str(source)], check=True, cwd=ROOT)
            results["compilation"].append(str(source.relative_to(ROOT)))
            print(f"PASS: compiled {source.relative_to(ROOT)}", flush=True)
        if not args.check_only:
            for source in fixtures:
                with ViewerSession(viewer, source, args.evidence) as session:
                    prefix = ROOT_TYPES[source.stem]
                    session.call("click_element", elementHandle=session.find(f"{prefix}::run-tests-button"))
                    report_handle = session.find(f"{prefix}::{REPORT_IDS.get(source.stem, 'report')}")
                    report_value = lambda: session.properties(report_handle).get("accessibleLabel", "")
                    session.wait_until(lambda: report_value().startswith(("PASS:", "FAIL:")))
                    report = report_value()
                    if not report.startswith("PASS:"):
                        raise AssertionError(f"{source.stem}: {report or 'Missing accessible test report'}")
                    results["contracts"].append({"fixture": source.stem, "result": report})
                    session.screenshot(f"{source.stem.lower()}-contracts.png")
                    print(report, flush=True)
                if source.stem in LIVE_CHECKS:
                    with ViewerSession(viewer, source, args.evidence) as session:
                        report = LIVE_CHECKS[source.stem](session)
                        results["interactions"].append({"fixture": source.stem, "result": report})
                        print(report, flush=True)
            if not args.fixture:
                with ViewerSession(viewer, catalog, args.evidence) as session:
                    report = check_catalog(session)
                    results["interactions"].append({"fixture": "Catalog", "size": [1200, 800], "result": report})
                    print(report, flush=True)
                compact_evidence = args.evidence / "catalog-compact"
                compact_evidence.mkdir(parents=True, exist_ok=True)
                with ViewerSession(viewer, catalog, compact_evidence,
                                   initial_properties={"viewport-width": 820, "viewport-height": 560}) as session:
                    report = check_catalog(session, expected_size=(820, 560))
                    results["interactions"].append({"fixture": "Catalog", "size": [820, 560], "result": report})
                    print(f"820×560: {report}", flush=True)
        results["status"] = "passed"
        return 0
    except (OSError, RuntimeError, AssertionError, TimeoutError, subprocess.CalledProcessError, KeyError, ValueError) as error:
        results["status"], results["error"] = "failed", str(error)
        print(f"FAIL: {error}", file=sys.stderr)
        return 1
    finally:
        (args.evidence / "result.json").write_text(json.dumps(results, indent=2) + "\n", encoding="utf-8")


if __name__ == "__main__":
    raise SystemExit(main())
