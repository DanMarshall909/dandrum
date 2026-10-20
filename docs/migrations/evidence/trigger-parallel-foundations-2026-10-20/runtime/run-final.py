import json, os, pathlib, subprocess, time
root=pathlib.Path('/tmp/dandrum-lanes/runtime')
evidence=pathlib.Path('/tmp/dandrum-lanes/evidence-runtime')
env=dict(os.environ,CARGO_TARGET_DIR='/tmp/dandrum-lanes/cache-runtime',CARGO_BUILD_JOBS='2')
checks=[
 ('11-clippy',['/home/dan/.cargo/bin/cargo','clippy','--manifest-path','src/trigger-host/Cargo.toml','--all-targets','--','-D','warnings']),
 ('12-coverage',['/home/dan/.cargo/bin/cargo','llvm-cov','--manifest-path','src/trigger-host/Cargo.toml','--lib','--test','runtime','--lcov','--output-path',str(evidence/'12-final.lcov')]),
 ('13-owner-tests',['/home/dan/.cargo/bin/cargo','test','--manifest-path','src/trigger-host/Cargo.toml']),
 ('14-format',['/home/dan/.cargo/bin/rustfmt','--check','--edition','2024','src/trigger-host/src/runtime.rs','src/trigger-host/tests/runtime.rs']),
]
for name,cmd in checks:
 start=time.monotonic()
 with (evidence/(name+'.log')).open('w') as output:
  result=subprocess.run(cmd,cwd=root,env=env,stdout=output,stderr=subprocess.STDOUT)
 receipt=dict(command=cmd,cwd=str(root),environment={k:env[k] for k in ['CARGO_TARGET_DIR','CARGO_BUILD_JOBS']},elapsed_seconds=round(time.monotonic()-start,3),exit_code=result.returncode)
 (evidence/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n')
 print(name,result.returncode,receipt['elapsed_seconds'],flush=True)
 if result.returncode:
  print((evidence/(name+'.log')).read_text()[-6000:],flush=True)
  break
