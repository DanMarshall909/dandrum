import json, os, subprocess
from pathlib import Path
root=Path('/tmp/dandrum-optimized-audit-20261007');build=Path('/tmp/dandrum-frontend-performance/build/demo-webview')
env=dict(os.environ);env['PATH']='/usr/bin:/bin:'+env['PATH']
for kind,old in [('resource',Path('/tmp/dandrum-resource-audit-20261007')),('latency',Path('/tmp/dandrum-latency-audit-20261007'))]:
 new=root/kind;(new/'probe.cpp').write_bytes((old/'probe.cpp').read_bytes())
 for name in ['compile-command.json','link-command.json']:
  args=json.loads((old/name).read_text())
  args=[a.replace('/tmp/dandrum-instrument-system-ui','/tmp/dandrum-frontend-performance').replace(str(old),str(new)) for a in args]
  (new/name).write_text(json.dumps(args,indent=2)+'\n')
  subprocess.run(args,cwd=build,env=env,check=True)
 print('built',kind,flush=True)
