"""One-off audit of the installed artifact produced by the maintained fixture."""
from pathlib import Path
import hashlib,json,os,subprocess,time,signal,sys
out=Path('/tmp/dandrum-lanes/evidence-trigger-clap-sourcefree');assert json.loads((out/'receipt.json').read_text())['exit']==0
f=json.loads((out/'fixture.json').read_text());installed=Path(f['fixture'])/'installed';artifact=installed/'DandrumTrigger.clap';assert artifact.is_file();audit=out/'release-audit';assert not audit.exists();audit.mkdir()
validator=Path('/tmp/dandrum-validator-0.4.1-qualified/clap-validator');expected='d9a78bfe781719de450bc5013141496eb13e1d9a538c67de1b8d325628670dbf';assert hashlib.sha256(validator.read_bytes()).hexdigest()==expected
commands=[('validator',[str(validator),'validate','--plugin-id','org.dandrum.trigger','--json','--jobs','1',str(artifact)]),('dynamic-links',['readelf','-d',str(artifact)]),('resolved-links',['ldd',str(artifact)]),('exported-symbols',['nm','-D',str(artifact)]),('normal-dependencies',['/home/dan/.cargo/bin/cargo','tree','--offline','--locked','--manifest-path',str(Path(f['source'])/'src/clap-adapter/Cargo.toml'),'--features','trigger','--edges','normal'])]
records=[]
for label,argv in commands:
 log=audit/(label+'.stdout');err=audit/(label+'.stderr');started=time.monotonic();reason=None
 with log.open('wb') as stdout,err.open('wb') as stderr:
  proc=subprocess.Popen(argv,cwd=installed,env=dict(os.environ,CARGO_NET_OFFLINE='true'),stdout=stdout,stderr=stderr,start_new_session=True)
  while proc.poll() is None:
   d=os.statvfs('/tmp');m=int(next(l.split()[1] for l in Path('/proc/meminfo').read_text().splitlines() if l.startswith('MemAvailable:')))*1024
   if d.f_bavail*d.f_frsize<1024**3 or m<4*1024**3 or time.monotonic()-started>600:
    reason='Fixed resource floor or bounded 600-second per-command limit';os.killpg(proc.pid,signal.SIGINT);break
   time.sleep(.5)
  code=proc.wait()
 r={'label':label,'argv':argv,'cwd':str(installed),'exit':code,'seconds':time.monotonic()-started,'stop_reason':reason,'stdout_sha256':hashlib.sha256(log.read_bytes()).hexdigest(),'stderr_sha256':hashlib.sha256(err.read_bytes()).hexdigest()};records.append(r);(audit/'commands.json').write_text(json.dumps(records,indent=2)+'\n')
 if code!=0:print(json.dumps(r));sys.exit(code if code>0 else 128-code)
r={'artifact':str(artifact),'artifact_sha256':hashlib.sha256(artifact.read_bytes()).hexdigest(),'artifact_bytes':artifact.stat().st_size,'validator_sha256':expected,'commands':records,'limits':'Validator target selection org.dandrum.trigger, no test exclusions; parse all outcomes/skips/warnings before any acceptance claim. Link/dependency inspection supplements absent-source build, not a GUI/native/DAW proof.'};(audit/'receipt.json').write_text(json.dumps(r,indent=2)+'\n');print(json.dumps(r))
