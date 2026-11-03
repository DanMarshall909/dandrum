# Named DAW acceptance, task 7.3

Dan authorized task 7.3 on 2 November 2026. The acceptance uses installed Trigger
CLAP instances in REAPER 7.84 Linux x86_64, revision `4b6323`, with the production
React editor, actual native file selection and XTest pointer input.

The complete qualification, oracle, failed-probe dispositions and portable raw
receipts are indexed in
[`trigger-clap-daw-acceptance-2026-11-02`](../../../../docs/migrations/evidence/trigger-clap-daw-acceptance-2026-11-02/README.md).
That record identifies the tested package, required REAPER configuration,
composite gesture evidence, fresh project restoration and signed renders.

This acceptance found two defects: pointer gestures used a nested control's
short display label instead of its enclosing public parameter ID, and the CLAP
adapter omitted live-user flags from editor automation events. Both repairs
have failing regression evidence and passing checks at their owning boundaries.

Tasks 8.1 and 8.2 remain separate. This record does not declare the CLAP beta
complete, qualify REAPER's default GTK configuration, or add Qtractor support.
