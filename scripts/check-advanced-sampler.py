#!/usr/bin/env python3
"""Compile and exercise the advanced sampler's native headless UI contracts."""
import argparse
import json
import os
from pathlib import Path
import subprocess
import sys

REPO = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(REPO / "tests/slint"))
from mcp_client import ViewerSession
from advanced_sampler_chrome_checks import header_controls, waveform_controls, frame_geometry
from slint_headless import prepare_headless, use_environment


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--viewer", default="slint-viewer")
    parser.add_argument("--evidence", type=Path, default=REPO / "build/advanced-sampler-evidence")
    parser.add_argument("--headless", action="store_true")
    parser.add_argument("--check-only", action="store_true")
    args = parser.parse_args()
    plan = prepare_headless(sys.argv[1:], os.environ)
    if plan and plan.command:
        return subprocess.run(plan.command, env=plan.environment).returncode
    with use_environment(plan):
        return run_checks(args)


def run_checks(args):
    entries = ["AdvancedSamplerFrame", "AdvancedSamplerChrome", "AdvancedSamplerWave", "AdvancedSamplerScene"]
    for entry in entries:
        subprocess.run([args.viewer, "--check", str(REPO / f"tests/slint/{entry}.slint")], check=True)
    if args.check_only:
        print(json.dumps({"compiled": len(entries), "runtime": False}))
        return 0
    args.evidence.mkdir(parents=True, exist_ok=True)
    results = []
    with ViewerSession(args.viewer, REPO / "tests/slint/AdvancedSamplerFrame.slint", args.evidence) as session:
        session.call("click_element", elementHandle=session.find("AdvancedSamplerFrameTest::run-tests-button"))
        result = session.properties(session.find("AdvancedSamplerFrameTest::report"))["accessibleLabel"]
        if not result.startswith("PASS:"):
            raise AssertionError(result)
        results.append(result)
    for name, check in [("AdvancedSamplerChrome", header_controls), ("AdvancedSamplerWave", waveform_controls),
                        ("AdvancedSamplerScene", lambda session: frame_geometry(session, 1200, 800))]:
        with ViewerSession(args.viewer, REPO / f"tests/slint/{name}.slint", args.evidence) as session:
            results.append(check(session))
    report = {"compiled": len(entries), "headless": args.headless or os.environ.get("DANDRUM_SLINT_HEADLESS") == "1", "results": results}
    (args.evidence / "chrome-report.json").write_text(json.dumps(report, indent=2) + "\n", encoding="utf-8")
    print(json.dumps(report))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
