#!/usr/bin/env python3
"""Keep retired graph and patch fields out of production Rust builds."""

from pathlib import Path
import re


ROOT = Path(__file__).resolve().parents[1] / "src/rust-engine/src"


def source(path: str) -> str:
    return (ROOT / path).read_text()


def body(path: str, declaration: str) -> str:
    text = source(path)
    start = text.index(declaration)
    opening = text.index("{", start)
    depth = 1
    for index in range(opening + 1, len(text)):
        depth += (text[index] == "{") - (text[index] == "}")
        if depth == 0:
            return text[opening + 1 : index]
    raise AssertionError(f"unclosed {declaration} in {path}")


def test_only(path: str, declaration: str) -> None:
    lines = source(path).splitlines()
    matches = [
        index
        for index, line in enumerate(lines)
        if re.match(r"(?:pub(?:\([^)]*\))? )?" + re.escape(declaration), line.lstrip())
    ]
    if len(matches) != 1:
        raise AssertionError(f"expected one {declaration!r} in {path}; found {len(matches)}")
    index = matches[0] - 1
    while index >= 0 and lines[index].strip().startswith(("#[", "///")):
        if lines[index].strip() == "#[cfg(test)]":
            return
        index -= 1
    raise AssertionError(f"{path}: {declaration} is not gated by #[cfg(test)]")


def absent(path: str, owner: str, field: str) -> None:
    if re.search(rf"(?m)^\s*{re.escape(field)}\s*:", body(path, owner)):
        raise AssertionError(f"{path}: {owner} still declares {field}")


def main() -> None:
    test_only("graph.rs", "enum ExecutionScope")
    test_only("graph.rs", "VoiceToGlobalDirectRouting {")
    test_only("graph.rs", "params: BTreeMap<String, String>")
    test_only("graph.rs", "pub fn params(&self)")
    test_only("compiled_patch.rs", "voice_node_indices: Vec<usize>")
    test_only("compiled_patch.rs", "global_node_indices: Vec<usize>")
    test_only("compiled_patch.rs", "audio_output_index: Option<usize>")
    test_only("graph_processor/render_plan.rs", "audio_output: Option<AudioOutputBinding>")
    test_only("graph_processor/realtime_graph_processor.rs", "allocator: VoiceAllocator")
    test_only("patch.rs", "pub struct PatchDocument {")
    test_only("builtins.rs", "fn audio_output_definition()")
    test_only("lib.rs", "pub(crate) mod voice_allocator;")
    absent("compiled_patch.rs", "pub struct CompiledNode {", "parameters")
    absent("builtins.rs", "pub struct BuiltInModuleDefinition {", "execution_scope")
    absent("kernel/document.rs", "pub struct KernelPatch {", "asset_bindings")
    print("Retired graph fields are absent or test-only.")


if __name__ == "__main__":
    main()
