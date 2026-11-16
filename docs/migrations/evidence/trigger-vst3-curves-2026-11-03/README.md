# VST3 standard curve queues — 3 November 2026

Partial Linux proof, live task3.1 boundary. Standard IParameterChanges and
IParamValueQueue are borrowed without COM reference-count traffic. At most23
unique public queues and1,024 combined points/notes are decoded into preallocated
storage. Complete validation, including output layout, precedes runtime changes.
The adapter passes borrowed normalized point slices to the shared runtime.

Five real dynamically loaded tests assert every signed ramp/jump sample, all23
queues, implicit previous endpoint,16 vs8+8 block equivalence, malformed getter/
count/ID/value/offset/duplicate rejection, combined1,024 acceptance vs1,025
rejection, cursor/scalar preservation and controls before same-frame notes. A
one-voice distinct-region witness proves host note order: final kick PCM+105,
versus first snare PCM-149, each divided by32768. An initial two-kick velocity
oracle was weak because velocity selects zones rather than scaling amplitude;
the independent reader identified it and the retained test was strengthened.

The calibrated real malloc/free sentinel observes0 allocations/0 frees over the
complete1,024-input curve callback, scalar flush, malformed batches and reset.
The stopped reentry parent now proves both point and event getters cannot
terminate their borrowed processor. Every direct ignored child is run by its
ordinary parent. Combined dynamic tests pass; strict Clippy passes. The private
freshly built CLAP product also passes20 dynamic signed timing/state/flush tests
(plus its parent-launched allocator child), preserving discrete behavior.

The point decoder was extracted into a private helper after behavior coverage;
the actual cdylib LLVM export measures68 calls and no uncovered decoder regions.
Whole audio coverage is339/348 lines(97.41%); defensive ownership/output/error
gaps remain visible. These accumulated TDD profiles are not final fresh-candidate
coverage. See verification.json for source, artifact and coverage identities.
Full LLVM JSON and every binary remain private.

Stopped normalized endpoint precision is the next explicit criterion: the
current physical scalar flush loses the original normalized endpoint at rounding
boundaries. Host feedback/gestures, portable state, actual React/Wry destruction
and recreation, installed REAPER, full official validator and exact combined
GPT-6 Astra review/publication remain mandatory. No task or whole-proof
completion is claimed here.
