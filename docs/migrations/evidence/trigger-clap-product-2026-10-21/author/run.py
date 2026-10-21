import os,sys,subprocess,time,json,pathlib,hashlib,shutil
root=pathlib.Path('/tmp/dandrum-lanes/runtime'); evidence=pathlib.Path('/tmp/dandrum-lanes/evidence-trigger-clap-product'); name=sys.argv[1]; cmd=sys.argv[2:]
env=os.environ.copy(); env.update(CARGO_TARGET_DIR='/tmp/dandrum-lanes/cache-runtime',CARGO_LLVM_COV_TARGET_DIR='/tmp/dandrum-lanes/cache-host-snapshot-cov',CARGO_LLVM_COV_BUILD_DIR='/tmp/dandrum-lanes/cache-host-snapshot-cov',CARGO_BUILD_JOBS='1',TMPDIR=str(evidence/'tmp'),DANDRUM_TRIGGER_CLAP_PLUGIN_PATH=str(evidence/'installed/DandrumTrigger.clap'),DANDRUM_TRIGGER_CLAP_AUDIT_EVIDENCE_DIR=str(evidence))

if name.startswith('cov-'):
 env['CARGO_TARGET_DIR']='/tmp/dandrum-lanes/cache-host-snapshot-cov'
 env['RUSTFLAGS']='-C instrument-coverage --cfg coverage'
 env['LLVM_PROFILE_FILE']=str(evidence/('trigger-%p-%m.profraw'))
env['LLVM_PROFILE_FILE']=str(evidence/(name+'-%p-%m.profraw'))
env['DANDRUM_CLAP_PLUGIN_PATH']=str(evidence/'installed/DandrumHeadlessProof.clap')
free=shutil.disk_usage(evidence).free; mem=int(next(l.split()[1] for l in open('/proc/meminfo') if l.startswith('MemAvailable:')))*1024
assert free>=1024**3 and mem>=4*1024**3,(free,mem)
sources={str(p.relative_to(root)):hashlib.sha256(p.read_bytes()).hexdigest() for owner in ('clap-adapter','trigger-host') for p in (root/'src'/owner).rglob('*') if p.is_file() and 'target' not in p.parts}
start=time.time(); log=evidence/(name+'.log')
with log.open('w') as f:
 p=subprocess.Popen(cmd,cwd=root,env=env,stdout=f,stderr=subprocess.STDOUT)
 while p.poll() is None:
  time.sleep(0.5)
  free=shutil.disk_usage(evidence).free; mem=int(next(l.split()[1] for l in open('/proc/meminfo') if l.startswith('MemAvailable:')))*1024
  if free<1024**3 or mem<4*1024**3:
   p.terminate(); raise RuntimeError(('guard',free,mem))
code=p.returncode
receipt=dict(command=cmd,cwd=str(root),environment={k:env[k] for k in ('CARGO_TARGET_DIR','CARGO_LLVM_COV_TARGET_DIR','CARGO_LLVM_COV_BUILD_DIR','CARGO_BUILD_JOBS','TMPDIR','DANDRUM_TRIGGER_CLAP_PLUGIN_PATH','DANDRUM_CLAP_PLUGIN_PATH','DANDRUM_TRIGGER_CLAP_AUDIT_EVIDENCE_DIR','LLVM_PROFILE_FILE','RUSTFLAGS') if k in env},sources=sources,exit=code,seconds=time.time()-start,disk_free=free,mem_available=mem)
(evidence/(name+'.json')).write_text(json.dumps(receipt,indent=2)+'\n'); print(name,code,round(receipt['seconds'],2)); print(log.read_text()[-3500:]); sys.exit(code)
