from pathlib import Path
import json
report = Path(__file__).with_name("15-frozen-coverage.lcov").read_text()
rows = []
for block in report.split("end_of_record"):
    if "SF:" not in block:
        continue
    source = next(line[3:] for line in block.splitlines() if line.startswith("SF:"))
    if not (source.endswith("/src/parameters.rs") or source.endswith("/src/runtime.rs")):
        continue
    names, starts, lines, raw = {}, {}, {}, []
    for line in block.splitlines():
        if line.startswith("FN:"):
            start, name = line[3:].split(",", 1)
            names[name] = int(start)
        elif line.startswith("DA:"):
            number, count, *_ = line[3:].split(",")
            lines[int(number)] = int(count)
    for line in block.splitlines():
        if line.startswith("FNDA:"):
            count, name = line[5:].split(",", 1)
            start = names[name]
            starts[start] = starts.get(start, 0) + int(count)
            raw.append(int(count))
    rows.append(dict(source=source, measured_lines=len(lines), covered_lines=sum(n > 0 for n in lines.values()), source_function_or_closure_starts=len(starts), covered_source_function_or_closure_starts=sum(n > 0 for n in starts.values()), raw_instantiations=len(raw), covered_raw_instantiations=sum(n > 0 for n in raw), uncovered_lines=[n for n, count in lines.items() if not count], uncovered_source_starts=[n for n, count in starts.items() if not count]))
print(json.dumps(rows, indent=2))
