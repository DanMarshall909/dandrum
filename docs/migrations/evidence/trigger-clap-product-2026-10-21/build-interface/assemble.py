"""One-off exact component assembly; requires explicit author handback marker."""
from pathlib import Path
import hashlib,json,subprocess,shutil,time,os,stat
out=Path('/tmp/dandrum-evidence/trigger-clap-cmake');assert (out/'author-terminal-handback.json').is_file()
author=Path('/tmp/dandrum-lanes/runtime');root=Path('/tmp/dandrum-lanes/integration');fp=Path('/tmp/dandrum-lanes/evidence-trigger-clap-product/source-freeze.json');freeze=json.loads(fp.read_text());base='72c58e0bcf308d5f181cf75f2875f76f3a1a3b4e'
def git(at,*args):return subprocess.check_output(['git',*args],cwd=at).decode().strip()
assert git(author,'rev-parse','HEAD')==git(root,'rev-parse','HEAD')==freeze['head']==base
assert not git(root,'diff','--cached','--name-only')
allowed={'CMakeLists.txt','README.md','tests/python/test_engine_only_build.py'}
assert set(git(root,'diff','--name-only').splitlines())==allowed
assert not git(root,'ls-files','--others','--exclude-standard')
readme=json.loads((out/'readme-draft.json').read_text());checkpoint=json.loads((out/'checkpoint.json').read_text())
for n in allowed:
 expected=readme['sha256'] if n=='README.md' else checkpoint['source'][n]['sha256'];assert hashlib.sha256((root/n).read_bytes()).hexdigest()==expected,n
paths=dict(freeze['owned_paths']);paths['openspec/changes/add-juce-free-trigger-clap-editor/design.md']=freeze['root_owned_design'];records=[];stamp=time.time_ns()
for n,r in sorted(paths.items()):
 src=author/n;dst=root/n;payload=src.read_bytes();assert hashlib.sha256(payload).hexdigest()==r['sha256'],n;assert oct(stat.S_IMODE(src.stat().st_mode))==r['mode'];assert src.stat().st_mtime_ns==r['mtime_ns'],n
 if dst.exists():assert subprocess.check_output(['git','show',base+':'+n],cwd=root)==dst.read_bytes(),n
 else:assert not dst.is_symlink()
 dst.parent.mkdir(parents=True,exist_ok=True);shutil.copy2(src,dst);os.utime(dst,ns=(stamp,stamp));assert dst.read_bytes()==payload
 records.append({'path':n,'sha256':r['sha256'],'mode':r['mode'],'original_mtime_ns':r['mtime_ns'],'assembled_mtime_ns':dst.stat().st_mtime_ns,'reason':'Fresh timestamp ensures reused Cargo detects current copied inputs; content/mode unchanged'})
record={'base':base,'author':str(author),'integration':str(root),'source_freeze_sha256':hashlib.sha256(fp.read_bytes()).hexdigest(),'paths':records,'root_owned':{n:hashlib.sha256((root/n).read_bytes()).hexdigest() for n in sorted(allowed)},'scope':'Combined headless Trigger source exact author bytes plus root CMake/Python/README and accepted design; no source/evidence deletion, no cache writes'};(out/'combined-ready.json').write_text(json.dumps(record,indent=2)+'\n');print(json.dumps({'assembled_paths':len(records),'receipt':str(out/'combined-ready.json')}))
