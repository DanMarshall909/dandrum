# Keep product control descriptions stable in a silent graph

This focused task5.3 prerequisite retains product scope/controlGroup with the
immutable public alias/range descriptors constructed from initially prepared
bindings. Live physical values remain owned by runtime atomics. A silent accepted
graph may route those controls to unused control_mixer inputs; it cannot silently
turn all sample-group controls into instrument controls. No protocol field,
resource owner, serialized-state format, queue or engine language change is added.
The existing nonempty-map requirement remains unchanged.

The new real-host test starts with literal signed kit[-1,-1] at physicalgain2,
prepares a resource-free silence/control graph preserving every root control and
alias, consumes the actual block-boundary swap on a scoped callback thread and
asserts zero output with zero allocations/deallocations. It then compares all23
public descriptions/applied values. GenuineRED reaches the silent callback but
loses group scope; minimalGREEN caches grouping beside the existing frozen ranges
and defaults, rather than recomputing it from a later graph. Initial bindings,
including misleading alias names and control fanout, remain covered by existing
snapshot tests. No alias-name heuristic supplies product grouping.

Formatting/strict host and all-target editor-adapterClippy pass. The complete
ordinary host owner passes192tests plus2doctests; focused parameter/snapshot/
selected-admission tests pass as well.3exact instrumented host test ELFs produce
3fresh external raw profiles. All changed executable source lines have nonzero
LLVM counts, including global/grouped/fanout paths. The broad parameter owner is
91/114functions,926/1227lines and1384/1757regions in this focused selection. This
is execution evidence, not full-owner or branch coverage. Exact per-object exports,
merged profiles, annotations and source hashes are retained; no cache/profile
cleaning or mutation occurs.

Production refactor review keeps grouping in the existing descriptor constructor
and projection; there is no second state owner or newly extracted module. Test
refactor review retains the existing real admission fixture and one new graph
transition, with independent signed baseline/zero/callback assertions. No broad
test helper or new maintained verification runner is introduced. The one-off
coverage script follows the previously calibrated show-env/profile/object flow.

[Binding](binding.json), [index](raw-index.json) and [raw](raw.tar.gz) retain exact
source overlays, RED source, command/cleanup/coverage evidence. Exact instrumented
objects remain in the externally bound archive (about84MiBcompressed), read back
against compiler artifact hashes; reusable caches remain untouched. Active strict
OpenSpec and631/392/239mapping checks pass with no task/scenario closure.

This is a local foundation checkpoint, not independent approval or native Empty
completion. The production New Empty menu is genuinely RED in its separately
retained copied-product fixture: the accepted kit remains revision0/history0.
Frontend Empty graph serialization and recreated selected import remain next;
current uncommitted CMake/native test changes are outside this focused commit.
The previous native product33af9b90 qualifies the published05b1048baseline, not
this changed Rust source. Rebuild/native/official/current-source release proof,
independent review and normal publication remain pending at the full Empty
boundary. Engine strictClippy129baselineFAIL remains explicit. Mainclean68dd655,
CLAPonly5/17OPEN, mutationexcluded, VST3/standalone deferred. Owned workers/scratch
are cleaned; disk-backed reusable caches/evidence remain, about96GiBfree.
