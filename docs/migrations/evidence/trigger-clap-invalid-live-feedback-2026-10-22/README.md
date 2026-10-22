# Invalid state preserves the live Trigger feedback path

Partial task 4.3 of `add-juce-free-trigger-clap-editor`, over published
`65cb337183edd6586db1965e88c331b77678cc04`. The existing native feedback/Undo
case now joins invalid CLAP state rollback to its real X11 production React
view and active audio endpoint. No production code, task checkbox, canonical
scenario or spec mapping changes.

After a real physical level0.5 edit and balanced ID3 begin/value/end, an empty
flush collects accepted automation credits. The case pins saved bytes, full
session, parameter DTO generation1/sequence1, normalized0.125/physical0.5,
structural history0/0 and the original child plus composite attachment identity.
With that same endpoint active, eight rejected payloads cover foreign version,
resource, same-count duplicate parameter, out-of-range tail, malformed JSON,
oversize, truncation and late invalidity after an earlier valid mapping/value
change. Every failure retains those accepted public values and the live view.
Opaque CLAP failures do not independently identify codec categories; existing
owner and dynamic-host tests prove those categories at their respective layers.

The same endpoint then processes MIDI36 at0 and physical2 at1. Both channels
assert `[-0.25,-716/32768]`, authenticated consumed feedback updates the actual
control, and observer checks reject polling and automation echo. The actual React
header Undo produces balanced ID3 automation back to physical1, sequence2 and
unchanged structural history. Subsequent offset3 output has leading zeros and
`[-0.5,-358/32768]`. Final child and attachment identities remain original.
No successful restore, reset or view replacement separates rejection probes
from feedback. This is discrete callback evidence, not device cadence proof.

The only maintained Rust change is `src/clap-adapter/tests/trigger_parameters.rs`,
SHA-256 `9cf57d3cace4db2b44679d93fb55845c4acef541ff375a26d2ea56458ff917b6`.
This characterizes existing production behavior and passed immediately; no
artificial RED or production repair is claimed. Table-driven candidates reuse
one assertion block. Assertion failure attempts audio return and GUI destruction
independently before resuming the original panic. No module is extracted.

| Root verification | Result |
| --- | --- |
| Maintained native fixtures and full CTest lane | 28/28, 44.483s |
| Fast real dispatch/timer cases | 13/13, 0.445s; unchanged bound library-test ELF, new runs |
| Exact formatter / strict all-target Clippy | Pass, 0.081s / 0.186s |
| Fresh test instrumentation | Pass, 52.273s; verified compiler and17 unchanged extern inputs |
| Exact instrumented native case | One pass, 1.610s; one new profile |
| LLVM merge/export/annotation | Exit0, warning-free, current source present |
| Five actual copied/instrumented artifact paths | No unresolved loader or JUCE dynamic dependency |
| Strict change / canonical / spec-coverage gates | Pass;51 specs,618 scenarios,379 mapped,239 ratchet backlog |

The fresh instrumented test ELF is `a704989a658c03891e2f44893c771f3e361586d5987a2cf398135321d429b7d9`;
sole new profile `42c076ac3d8e009411cc862e920ef4b67de7beab18045f88df17d6eb14836543`.
Root ordinary test `bf762ef233d147e0b2faa9cd365b8af2c60d9b6e1fa768c7821413633fa6bdb2` remains separately bound
from the copied DSO `e9200fbb0bdc714e6f6d3950aefea03b1025302a328374b31310a45e745b0741` and production app `4b9ce5a3…`.
Cargo source/copy identities and all resource hashes are checked. Full commands,
environment, source-before/after digests, tool/dependency hashes, actual host PID,
resource samples and outputs are in indexed raw archives. The compiler PID is
namespace-local and is not the native host PID.

The exact test's normal added rejection and continuation assertions execute.
New assertion-failure cleanup lines1044–1058 do not execute; existing feedback
failure/timeout lines1073–1076 also remain unhit. These limits are explicit in
`root-changed-scope-coverage.json`. Whole parent-file totals are472/1264 lines,
912/2526 regions and43/104 functions because this focused run excludes many
unrelated tests. Those totals are not whole-owner or branch coverage. Successful
cleanup does not prove exceptional cleanup. Mutation remains expressly excluded.

Author final exact/native/formatter/Clippy receipts and preliminary pre-correction
source/run are retained separately. Root initially counted directories as cache
files and rejected its read-only admission before integration; correcting the
count confirmed1766 regular files. No cache or source repair was involved.
Prior eight-case headless build evidence is qualified reuse:76 extant compiler
inputs rehash equal, with one intentionally removed asset limitation. The current
test lies outside those77 product inputs; this is no fresh source-free GUI build.

Main remains clean at68dd655. Reusable caches stay; actual native socket access
uses the existing X11 session. Heavy/native operations serialize under1GiB disk
and4GiB available-memory floors. The maintained preparer explicitly uses Cargo
jobs2; direct compiler commands use one job. Maintained GUI-product wiring,
fresh real-bundle source-free proof, validator assessment and final mappings stay
open, followed by the full beta's VST3, standalone, DAW, installed/offline and
physical/native gates. Review/publication receipts belong to the sole tracker.

- `author-raw.tar.gz`: `cc8c1490813d1516b43adcfa146513b72f454af851d5323a96ed5764cb7a70b0`, 49 byte-verified indexed members.
- `root-raw.tar.gz`: `a55e368c0220a9423f9e098a9b8b788be1a399f964eb1dabf91f688f60d813d1`, 119 byte-verified indexed members.
