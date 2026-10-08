"""Repository-owned command-line entrypoint for Dandrum developer demos."""
import json
import os
import re
import shlex
import subprocess
import sys
from pathlib import Path


def checkouts(root):
    try:
        result = subprocess.run(["git", "-C", str(root), "worktree", "list", "--porcelain"],
                                check=True, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, text=True)
        paths = {Path(field[9:]) for field in result.stdout.splitlines()
                 if field.startswith("worktree ")}
        return [root] + sorted(path for path in paths if path != root
                               and (path / "src/rust-engine/Cargo.toml").is_file())
    except (OSError, subprocess.CalledProcessError):
        return [root]


def source_for(demo, roots):
    for root in roots:
        if not (root / demo["source"]).is_file():
            continue
        if demo["kind"] == "native":
            cmake = root / "CMakeLists.txt"
            target = demo["target"].removesuffix("_Standalone")
            if not cmake.is_file() or not re.search(r"(?m)^[ \t]*(?:juce_add_(?:plugin|console_app|gui_app)|add_executable)\(\s*" + re.escape(target) + r"(?:\s|\))", cmake.read_text()):
                continue
        return root
    return None


def run(command, cwd, env=None):
    print("+ " + shlex.join(map(str, command)), flush=True)
    subprocess.run(list(map(str, command)), cwd=cwd, env=env, check=True)


def prepare_package(package):
    installed = package / "node_modules/.package-lock.json"
    if not installed.is_file() or (package / "package-lock.json").stat().st_mtime > installed.stat().st_mtime:
        run(["npm", "ci"], package)


def build_configuration(build):
    cache = build / "CMakeCache.txt"
    if not cache.is_file():
        return ""
    values = dict(re.findall(r"^(CMAKE_BUILD_TYPE|CMAKE_CONFIGURATION_TYPES):[^=]*=(.*)$", cache.read_text(), re.M))
    configurations = [value for value in values.get("CMAKE_CONFIGURATION_TYPES", "").split(";") if value]
    if configurations:
        return "Release" if "Release" in configurations else configurations[0]
    return values.get("CMAKE_BUILD_TYPE", "")


def launch(demo, root, args):
    print(f"Launching {demo['name']} from {root}", flush=True)
    if demo["kind"] == "web":
        package = (root / demo["source"]).parent
        prepare_package(package)
        run(["npm", "run", "dev", "--", *args], package)
        return
    sampler = root / "web/sampler/package.json"
    if sampler.is_file() and not demo.get("nativeOnly"):
        prepare_package(sampler.parent)
    build = root / demo.get("buildDirectory", "build/demo-webview" if demo.get("react") else "build")
    local_cmake = Path.home() / ".local/bin/cmake"
    cmake = str(local_cmake) if local_cmake.is_file() else "cmake"
    env = dict(os.environ)
    if sys.platform.startswith("linux"):
        env["PATH"] = "/usr/bin:/bin:" + env.get("PATH", "")
    configure = [cmake, "-S", root, "-B", build]
    if demo.get("nativeOnly"):
        configure.append("-DDANDRUM_NATIVE_ONLY=ON")
    elif demo.get("react"):
        configure.append("-DDANDRUM_NATIVE_ONLY=OFF")
    configure.extend(f"-D{option}" for option in demo.get("cmakeOptions", []))
    run(configure, root, env)
    configuration = build_configuration(build)
    run([cmake, "--build", build, "--target", demo["target"], "--config", configuration or "Release"], root, env)
    parts = Path(demo["artifact"]).parts
    artifact = build / parts[0] / configuration / Path(*parts[1:])
    candidates = [artifact, artifact.with_name(f"{artifact.name}.exe"),
                  artifact.parent / f"{artifact.name}.app/Contents/MacOS/{artifact.name}"]
    executable = next((path for path in candidates if path.is_file()), None)
    if executable is None:
        raise RuntimeError(f"Built {demo['target']} but missing artifact: {artifact}")
    run([executable, *args], root)


def invoke(args, root):
    demos = json.loads((root / "scripts/demos.json").read_text())
    demo = next((demo for demo in demos if args and demo["name"] == args[0]), None)
    unknown = args and args[0] not in {"--list", "--help", "-h"} and demo is None
    if unknown:
        print(f"Unknown demo: {args[0]}", file=sys.stderr)
    roots = [root] if unknown else checkouts(root)
    if demo:
        source = source_for(demo, roots)
        if source is None:
            print(f"Demo {demo['name']} unavailable: need {demo['source']} in this checkout or a registered Dandrum worktree.", file=sys.stderr)
            return 1
        forwarded = args[1:]
        if forwarded[:1] == ["--"]:
            forwarded = forwarded[1:]
        launch(demo, source, forwarded)
        return 0
    print("Usage: ./demo <name> [-- demo arguments]\n")
    for demo in demos:
        source = source_for(demo, roots)
        print(f"  {demo['name']:16} {demo['description']}\n{'':19} {source or 'unavailable'}")
    return 2 if unknown else 0


def main(args, root=Path(__file__).resolve().parents[1]):
    try:
        return invoke(args, root)
    except subprocess.CalledProcessError as error:
        print(f"Demo command failed: {error}", file=sys.stderr)
        return error.returncode if error.returncode > 0 else 128 - error.returncode
    except OSError as error:
        print(f"Cannot launch demo: {error}. See README.md for prerequisites.", file=sys.stderr)
        return 127
    except (ValueError, RuntimeError) as error:
        print(f"Cannot launch demo: {error}", file=sys.stderr)
        return 1
    except KeyboardInterrupt:
        print("Demo interrupted.", file=sys.stderr)
        return 130


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
