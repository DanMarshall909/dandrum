"""Archived focused pilot: python selected-mutation-pilot.py /path/to/dandrum."""
import json, shutil, subprocess, sys, tempfile
from pathlib import Path
root=Path(sys.argv[1]).resolve()
plan=json.loads(Path(__file__).with_name('selected-mutations-plan.json').read_text())
results=[]
with tempfile.TemporaryDirectory(prefix='dandrum-ui-mutants-') as directory:
 work=Path(directory)
 for name in ['web/shared','web/sampler/src','tests/js']:
  target=work/name;target.mkdir(parents=True)
  for source in (root/name).glob('*.mjs'):shutil.copy2(source,target/source.name)
 (work/'web/sampler/node_modules').symlink_to(root/'web/sampler/node_modules',target_is_directory=True)
 for item in plan:
  original=(root/item['module']).read_text()
  assert original.count(item['before'])==1,item['mutation']
  (work/item['module']).write_text(original.replace(item['before'],item['after']))
  run=subprocess.run(['node','--test',item['test']],cwd=work,capture_output=True,text=True,timeout=20)
  assert run.returncode!=0 and 'not ok' in run.stdout,(item['mutation'],run.stdout,run.stderr)
  failure='\n'.join(line for line in run.stdout.splitlines() if 'not ok' in line or 'failureType:' in line or 'error:' in line)
  results.append({'module':item['module'],'mutation':item['mutation'],'result':'caught','failure':failure})
  print(item['mutation'],'caught',flush=True)
  (work/item['module']).write_text(original)
print(json.dumps(results,indent=2))
