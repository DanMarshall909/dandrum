# Build-fixture snapshot cleanup

The source-free build fixture retained every fresh source snapshot. Registering unittest class cleanup immediately after snapshot creation now removes only that snapshot after successful tests, test failures or setup exceptions. Shared Cargo targets and raw logs remain outside it.

The maintained fast regression runs the real fixture lifecycle against small local source inputs. It first failed in all three terminal cases (RED), then passed after the one-line owner change. Success and test-failure cases include a symlink to the shared target; dependency and log sentinels survive. The Linux-only fixture restriction is preserved. CMake registers the regression as `engine-build-fixture-cleanup` in the fast lane.

Final ordinary discovery, final CTest registration/execution, and two existing real CMake boundary methods pass. The latter leave the pre-existing snapshot inventory and shared target identity unchanged. Python trace executes the new cleanup registration three times. The first trace entrypoint discovered zero tests; that attempt is retained and rejected, with corrected source-bound coverage recorded separately. This is statement execution plus explicit filesystem assertions, not full-owner coverage.

`binding.json` hashes the three maintained inputs. `raw.tar.gz` and `raw-index.json` retain exact RED/GREEN commands, timings, logs, owner preimage, original RED test, final relevant annotated source, CMake inventory and disposable-output cleanup receipts. The fresh configure trees were removed after retaining their results. Existing historical fixtures and reusable caches remain.

This fixes ordinary unittest lifecycle cleanup. Abrupt process death, cancellation supervision and historical recovery remain separate work. Mutation remains excluded; no Rust, product, task, canonical specification or scenario mapping changed, and the full beta remains open.
