# Trigger beta verification storage

The first JUCE-free beta is CLAP-only. Mutation testing is excluded. Keep builds
and coverage on physical disk, with compatible Cargo targets and dependencies
reused outside disposable run directories. Before a heavy run, check disk space,
target sizes and `MemAvailable`; keep at least 8 GiB free disk and 8 GiB available
RAM, with one heavy job at a time. The helper below does not enforce admission or
evict caches. Linux's reclaimable filesystem page cache needs no ramdisk.

From the selected worktree, give each verification command a new run directory
on physical disk. Its parent must already exist:

```sh
python3 -B scripts/run-owned-verification.py run \
  --directory /tmp/dandrum-evidence/my-unique-run -- \
  python3 -B tests/python/test_engine_only_build.py EngineOnlyBuildTest
```

The helper owns only that run's `scratch/`. It supplies `TMPDIR` and
`DANDRUM_VERIFICATION_SCRATCH` to the command. The source-free build fixture puts
fresh source/configuration snapshots there; its reusable Cargo target and command
logs stay under the worktree's `build/engine-only-regression/`. For other tools,
explicitly put disposable outputs in scratch and reusable targets outside it.
The helper cannot discover arbitrary output paths chosen by a command.

`stdout.log`, `stderr.log`, ownership records and cleanup receipts remain outside
scratch after success, command failure, launch failure or cancellation. Command
exit values and signals are retained; receipt allocation counts use filesystem
blocks, including directories and symlinks without following symlink targets.
The free-space before/after values are observations of the whole filesystem and
may include concurrent activity. They are not a promise of exact net free space.

If the controller dies, recover the exact run in the same user, boot and PID
namespace:

```sh
python3 -B scripts/run-owned-verification.py recover \
  --directory /tmp/dandrum-evidence/my-unique-run
```

A live controller holds an exclusive recovery lock. The Linux child-subreaper
guardian retains descendants even if they detach or clear their environment.
Stop and recovery signal verified process identities through pidfds, reap owned
children, and require the guardian's closure proof before deleting scratch.
A stop can race with command launch; any admitted command must close before
successful cleanup. This is not an atomic prohibition on launching immediately
after controller death. A failed stop keeps the guardian alive for a later retry.

Unknown or replaced paths, changed ownership records, unavailable context, or
guardian death without a closure proof refuse deletion. Inspect such runs rather
than treating them as cleaned. Bootstrap crashes before a complete ownership
record also require inspection. This is an operational ownership boundary, not
isolation against malicious commands running as the same user.

Completed recovery is idempotent. Known command outcomes and the initial scratch
allocation survive interrupted deletion; an outcome not observed before recovery
remains unknown. Preserve raw coverage profiles and required logs/evidence before
placing anything in scratch. After a run, archive needed receipts and remove only
proven owned disposable outputs. Reusable cache deletion still needs explicit
direction and an explained rebuild cost.

The maintained Linux CTest selections are `owned-verification-fast` and
`owned-verification-slow`. The slow selection exercises the real pending-stop
deadline and retry. `engine-build-fixture-cleanup` protects ordinary class cleanup
and supervised snapshot placement. These tests use isolated temporary fixtures;
they do not run mutation or evict real caches.
