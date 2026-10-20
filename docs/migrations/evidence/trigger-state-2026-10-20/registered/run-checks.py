import hashlib
import json
import os
from pathlib import Path
import subprocess
import sys
import time

ROOT = Path('/tmp/dandrum-lanes/runtime')
OUT = Path('/tmp/dandrum-evidence/state-review')
env = dict(os.environ, PATH='/home/dan/.local/bin:/home/dan/.cargo/bin:/usr/bin:/bin',
           CARGO_TARGET_DIR='/tmp/dandrum-lanes/cache-runtime', CARGO_BUILD_JOBS='2')
checks = {
    'session-coverage': ['/home/dan/.cargo/bin/cargo', 'llvm-cov', '--manifest-path',
        'src/instrument-session/Cargo.toml', '--lcov', '--output-path', str(OUT / 'session-coverage.lcov')],
    'registered-owners': ['/home/dan/.local/bin/ctest', '--test-dir', 'build/parallel-final',
        '--output-on-failure', '-R', '^(trigger-host-tests|instrument-session-tests)$'],
    'source-free-trigger': ['/usr/bin/python3', 'tests/python/test_engine_only_build.py',
        'EngineOnlyBuildTest.test_trigger_editor_scaffold_stages_copied_resources_without_host_sources'],
    'source-free-trigger-corrected': ['/usr/bin/python3', 'tests/python/test_engine_only_build.py',
        'ClapProofBuildTest.test_trigger_editor_scaffold_stages_copied_resources_without_host_sources'],
}
records = []
for name in sys.argv[1:]:
    argv = checks[name]
    start = time.time()
    with (OUT / (name + '.log')).open('wb') as log:
        result = subprocess.run(argv, cwd=ROOT, env=env, stdout=log, stderr=subprocess.STDOUT)
    record = dict(name=name, argv=argv, cwd=str(ROOT), exit_code=result.returncode,
                  start_unix=start, elapsed_seconds=round(time.time()-start, 6),
                  environment={k:env[k] for k in ('PATH','CARGO_TARGET_DIR','CARGO_BUILD_JOBS')},
                  log_sha256=hashlib.sha256((OUT/(name+'.log')).read_bytes()).hexdigest())
    records.append(record)
    (OUT / ('run-'+'-'.join(sys.argv[1:])+'.json')).write_text(json.dumps(records, indent=2)+'\n')
    print(json.dumps(record), flush=True)
    if result.returncode:
        sys.exit(result.returncode)
