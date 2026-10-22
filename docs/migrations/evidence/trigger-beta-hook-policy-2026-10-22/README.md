# Trigger beta normal-hook policy

Task 1.2 changes ordinary pushes to run the Rust unit gate and explicitly report the recorded mutation exclusion. Unit failures still stop the push. Strict engine coverage in pre-commit is unchanged. No hook is bypassed.

Four behavior tests invoke the real hook in real temporary Git repositories with paths containing spaces and a recording Cargo stand-in. They cover changed Rust, non-Rust changes, absent upstream and unit failure. The preimage produces three expected failures; the final hook passes all four. Bash DEBUG tracing observes all six executable command lines, without claiming branch coverage or a real Cargo invocation.

Comparable ordinary runs stay below 0.3 seconds. CTest now registers this fast test before the engine-only return, making the normal test available to the JUCE-free build. The corrected real configure/inventory/CTest sequence proves one selection and four passing cases. An earlier empty CTest selection and a native configure linker failure are retained as unsuccessful checks. Their disposable configure trees were removed.

The direct unit gate needs no helper extraction. The four tests retain distinct outcomes and reuse their existing Git fixture. No new module or Rust behavior is introduced. See binding.json for source hashes and limitations, and raw-index.json for every retained raw member. Mutation remains excluded; the full beta remains open.
