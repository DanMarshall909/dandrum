---
name: openspec-route-task
description: Route the next OpenSpec task by its [frontier], [standard], or [mechanical] model-tier tag. Use when choosing a model, handing off a task, starting tiered OpenSpec work, or deciding whether the current model should continue.
license: MIT
compatibility: Requires openspec CLI and model-tier tags defined by the change design.
metadata:
  author: dandrum
  version: "1.0"
---

# Route An OpenSpec Task

Select the next pending task, enforce its minimum model tier, and produce an execution or handoff decision. This skill
routes work; it does not pretend to change the active OpenCode model.

## Input

Accept an optional OpenSpec change name. If omitted:

- Infer it from conversation context when unambiguous.
- Auto-select when exactly one active change exists.
- Otherwise run `openspec list --json` and ask the user to select a change.

## Routing Workflow

1. Announce `Using change: <name>` and explain that another change can be selected with `/opsx-route <other>`.
2. Run:

   ```bash
   openspec status --change "<name>" --json
   openspec instructions apply --change "<name>" --json
   ```

3. Apply the same blocked, all-done, and workspace guards as `openspec-apply-change`.
4. Read every file listed in `contextFiles`, including the design that defines the model-tier policy.
5. Select the first pending task in the ordered task list. Do not skip a pending task merely to find a cheaper tier.
6. Parse exactly one leading tier tag from its description:

   - `[frontier]`
   - `[standard]`
   - `[mechanical]`

7. If the first pending task has no recognized tag, has multiple tags, or conflicts with the design policy, stop and ask
   for the planning artifact to be corrected. Do not guess a tier.
8. Evaluate whether the current model is explicitly known to satisfy the tier. Never infer capability from marketing
   names, parameter count, provider, or local/cloud location alone.

## Tier Decisions

### Frontier

Use a frontier reasoning/coding model as primary owner. The model must read the full context, reconcile cross-cutting
invariants, implement with TDD, and verify the complete affected boundary.

- Continue only when the current model is explicitly known to be frontier-capable.
- Otherwise stop before editing and emit a frontier handoff packet.
- Do not delegate architecture, unsafe FFI, realtime ownership, or destructive cleanup to a smaller subagent.

### Standard

Use a capable coding model when architecture and acceptance behavior are settled.

- A frontier model may continue because tiers are minimums, but point out that a standard model is the cost-efficient
  default.
- A standard model may implement directly with TDD and focused/full relevant tests.
- Escalate before editing if any D11 trigger appears.

### Mechanical

Use a fast smaller coding model only for deterministic transforms or verification with explicit expected output.

- Confirm all prerequisite capability tasks are complete.
- Confirm one representative fixture or command path has already been proven when the task is a bulk migration.
- Work in small batches and validate after every batch.
- Stop and escalate rather than diagnosing non-obvious failures or inventing exceptions.

## Mandatory Frontier Escalation

Regardless of the task's original tag, stop and route to frontier when work exposes any of these:

- A compiled-representation or public-ABI decision
- Unsafe-code, pointer-lifetime, ownership, or realtime-allocation reasoning
- An unspecified subsystem ownership boundary
- Conflicting or insufficient acceptance criteria
- A required compatibility behavior not stated in the artifacts
- A mechanical transform that cannot preserve behavior uniformly
- A non-obvious verification failure whose fix changes architecture or semantics

When escalating, leave the task unchecked and report the concrete trigger with file references.

## Handoff Packet

When the current model should not execute the task, output:

```text
OpenSpec change: <name>
Schema: <schema>
Progress: <complete>/<total>
Next task: <id and exact description>
Required tier: <frontier|standard|mechanical>
Why this tier: <specific D11 rationale>
Context files: <paths from apply instructions>
Required verification: <tests/checks named by task and repo policy>
Escalation triggers: <relevant triggers>
Suggested invocation: select a model satisfying <tier>, then run /opsx-apply <name>
```

Do not mark the task complete, edit implementation files, or create a commit during routing-only handoff.

## Execution Handoff To Apply

If the current model is suitable and the user asked to implement rather than only classify:

1. State the routing decision and selected tier.
2. Load `openspec-apply-change`.
3. Implement only while the task remains within its tier assumptions.
4. Re-run this routing check before moving to the next pending task because adjacent tasks may use different tiers.

## Output

Always report:

- Change and next pending task
- Required tier and rationale
- Whether the current model may continue
- Any prerequisite or escalation concern
- Exact next action
