"""One-off guarded execution of the maintained copied-product fixture after assembly."""
from pathlib import Path
import os,sys,json,subprocess,time,hashlib,tarfile,signal
root=Path('/tmp/dandrum-lanes/integration');out=Path('/tmp/dandrum-lanes/evidence-trigger-clap-sourcefree')
assert (Path('/tmp/dandrum-evidence/trigger-clap-cmake')/'combined-ready.json').is_file(), 'Do not run before assembled source and explicit author cache handback'
assert not out.exists();out.mkdir();(out/'tmp').mkdir();(out/'allocator-audit').mkdir()
work=root/'build/engine-only-regression';target=work/'cargo-target';assert target.exists() and target.resolve().stat().st_dev==Path('/tmp').stat().st_dev
oldlogs=[p for p in sorted(work.glob('*.log')) if p.is_file()]
with tarfile.open(out/'preexisting-driver-logs.tar.gz','w:gz') as tf:
 for p in oldlogs:tf.add(p,arcname=p.name,recursive=False)
(out/'preexisting-driver-logs.json').write_text(json.dumps({p.name:hashlib.sha256(p.read_bytes()).hexdigest() for p in oldlogs},indent=2)+'\n')
env=dict(os.environ,PATH='/home/dan/.local/bin:/home/dan/.cargo/bin:/usr/bin:/bin',CARGO_NET_OFFLINE='true',CARGO_INCREMENTAL='0',CARGO_BUILD_JOBS='1',CARGO_TARGET_DIR=str(target),CARGO_LLVM_COV_TARGET_DIR=str(target),CARGO_LLVM_COV_BUILD_DIR=str(target),CARGO_PROFILE_DEV_DEBUG='0',CARGO_PROFILE_TEST_DEBUG='0',TMPDIR=str(out/'tmp'),DANDRUM_TRIGGER_CLAP_AUDIT_EVIDENCE_DIR=str(out/'allocator-audit'))
for key in ('CARGO_TARGET_DIR','CARGO_LLVM_COV_TARGET_DIR','CARGO_LLVM_COV_BUILD_DIR','TMPDIR'):
 assert Path(env[key]).resolve().stat().st_dev==Path('/tmp').stat().st_dev
names=['CMakeLists.txt']
for directory in ('src','tests','examples','schema','scripts','spikes'):
 names += [str(p.relative_to(root)) for p in (root/directory).rglob('*') if p.is_file() and not any(x in ('target','build','__pycache__') for x in p.relative_to(root).parts)]
source={n:hashlib.sha256((root/n).read_bytes()).hexdigest() for n in sorted(set(names))};(out/'source-manifest.json').write_text(json.dumps(source,indent=2)+'\n')
# Wrapper records exact unmodified maintained command calls and their raw logs.
child=out/'execute.py';child.write_text('''from pathlib import Path
import importlib.util,sys,unittest,time,json,hashlib,trace,ast,os
root=Path('/tmp/dandrum-lanes/integration');out=Path('/tmp/dandrum-lanes/evidence-trigger-clap-sourcefree');p=root/'tests/python/test_engine_only_build.py'
spec=importlib.util.spec_from_file_location('trigger_copied_fixture',p);m=importlib.util.module_from_spec(spec);sys.modules[spec.name]=m;spec.loader.exec_module(m)
original=m.EngineBuildFixture.command;calls=[]
def recorded(self,label,args):
 started=time.monotonic();code,text=original(self,label,args);log=out/(label+'.log');log.write_text(text);r={'label':label,'argv':args,'cwd':os.getcwd(),'exit':code,'seconds':time.monotonic()-started,'log':log.name,'sha256':hashlib.sha256(log.read_bytes()).hexdigest()};calls.append(r);(out/'commands.json').write_text(json.dumps(calls,indent=2)+'\\n');return code,text
m.EngineBuildFixture.command=recorded
suite=unittest.defaultTestLoader.loadTestsFromNames(['ClapProofBuildTest.test_trigger_editor_scaffold_stages_copied_resources_without_host_sources'],m);t=trace.Trace(count=True,trace=False,ignoredirs=['/usr']);result=t.runfunc(unittest.TextTestRunner(verbosity=2).run,suite)
counts=t.results().counts;owner=next(n for n in ast.parse(p.read_text()).body if isinstance(n,ast.ClassDef) and n.name=='ClapProofBuildTest');method=next(n for n in owner.body if isinstance(n,ast.FunctionDef) and n.name=='test_trigger_editor_scaffold_stages_copied_resources_without_host_sources');lines=sorted({n.lineno for n in ast.walk(method) if isinstance(n,ast.stmt) and n is not method});c={str(n):counts.get((str(p),n),0) for n in lines};(out/'fixture-statement-counts.json').write_text(json.dumps({'scope':'Owning copied-product test body statement starts, not branch/CMake execution coverage','counts':c},indent=2)+'\\n');(out/'fixture.json').write_text(json.dumps({'fixture':str(m.ClapProofBuildTest.fixture_root),'source':str(m.ClapProofBuildTest.source),'cargo_target':str(m.ClapProofBuildTest.cargo_target),'testsRun':result.testsRun,'successful':result.wasSuccessful()},indent=2)+'\\n');assert result.wasSuccessful() and result.testsRun==1
''')
argv=['python3',str(child)]
def resources():
 d=os.statvfs('/tmp');mem=int(next(l.split()[1] for l in Path('/proc/meminfo').read_text().splitlines() if l.startswith('MemAvailable:')))*1024
 return {'disk_free_bytes':d.f_bavail*d.f_frsize,'memory_available_bytes':mem}
def low(r):return r['disk_free_bytes']<1024**3 or r['memory_available_bytes']<4*1024**3
admission=resources();assert not low(admission),admission
samples=[];start=time.monotonic();reason=None
with (out/'output.log').open('wb') as log:
 proc=subprocess.Popen(argv,cwd=root,env=env,stdout=log,stderr=subprocess.STDOUT,start_new_session=True)
 while proc.poll() is None:
  r=resources();samples.append(r)
  if low(r):reason='Fixed resource floor reached; stop only owned process group, preserve caches';os.killpg(proc.pid,signal.SIGINT);break
  time.sleep(.5)
 code=proc.wait()
assert all(hashlib.sha256((root/n).read_bytes()).hexdigest()==h for n,h in source.items()),'Root source changed during integration gate'
r={'argv':argv,'cwd':str(root),'exit':code,'seconds':time.monotonic()-start,'pid_and_pgid':proc.pid,'stop_reason':reason,'environment':{k:env[k] for k in ('PATH','CARGO_NET_OFFLINE','CARGO_INCREMENTAL','CARGO_BUILD_JOBS','CARGO_TARGET_DIR','CARGO_LLVM_COV_TARGET_DIR','CARGO_LLVM_COV_BUILD_DIR','CARGO_PROFILE_DEV_DEBUG','CARGO_PROFILE_TEST_DEBUG','TMPDIR','DANDRUM_TRIGGER_CLAP_AUDIT_EVIDENCE_DIR')},'resource_first':admission,'resource_min':{k:min(x[k] for x in samples) for k in admission},'source_inputs':len(source),'source_manifest_sha256':hashlib.sha256((out/'source-manifest.json').read_bytes()).hexdigest(),'log_sha256':hashlib.sha256((out/'output.log').read_bytes()).hexdigest(),'profile_qualification':'Real release product build; debug test/dependency symbols disabled and incremental disabled to bound disk, debug assertions unchanged. Existing target reused; no deletion/new Cargo root. Owning ordinary/LLVM profile evidence separate.'};(out/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r));sys.exit(code if code>=0 else 128-code)
