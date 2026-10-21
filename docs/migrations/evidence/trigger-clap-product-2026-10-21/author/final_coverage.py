import pathlib,subprocess,re,json,hashlib,time
r=pathlib.Path('/tmp/dandrum-lanes/evidence-trigger-clap-product'); root=pathlib.Path('/tmp/dandrum-lanes/runtime'); tool=pathlib.Path('/home/dan/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/lib/rustlib/x86_64-unknown-linux-gnu/bin')
phases=['cov-62-final-host','cov-72-final-adapter'];raw=sorted(p for name in phases for p in r.glob(name+'-*.profraw')); binaries=[]
for name in phases:
 binaries.extend(pathlib.Path(x) for x in re.findall(r'Running .* \((/[^\n)]*)\)',(r/(name+'.log')).read_text()))
binaries=list(dict.fromkeys(binaries)); binaries.extend([r/'objects/Trigger-instrumented-final.clap',r/'objects/HeadlessProof-instrumented.clap'])
files=[str(p) for owner in ['trigger-host','clap-adapter'] for p in sorted((root/'src'/owner/'src').glob('*.rs'))]
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
objects=[];by_id={}
for p in binaries:
 notes=subprocess.check_output(['readelf','-n',str(p)],text=True);bid=re.findall(r'Build ID: (\w+)',notes)
 entry={'path':str(p),'sha256':sha(p),'build_ids':bid,'size':p.stat().st_size};objects.append(entry)
 for b in bid:by_id.setdefault(b,[]).append(str(p))
compiler_objects=[]
for p in pathlib.Path('/tmp/dandrum-lanes/cache-host-snapshot-cov/debug/deps').glob('*.so'):
 notes=subprocess.check_output(['readelf','-n',str(p)],text=True);bid=re.findall(r'Build ID: (\w+)',notes)
 entry={'path':str(p),'sha256':sha(p),'build_ids':bid,'size':p.stat().st_size,'scope':'compiler proc-macro DSO, not an executed owner test'};compiler_objects.append(entry)
 for b in bid:by_id.setdefault(b,[]).append(str(p))
owner_ids={b for x in objects for b in x['build_ids']}
profiles=[]
for p in raw:
 result=subprocess.run([str(tool/'llvm-profdata'),'show','--binary-ids',str(p)],capture_output=True,text=True,check=True);(r/(p.name+'.show.txt')).write_text(result.stdout+result.stderr)
 ids=re.findall(r'^([0-9a-f]{40,})$',result.stdout,re.M)
 profiles.append({'path':str(p),'sha256':sha(p),'binary_ids':ids,'resolved_objects':{b:by_id.get(b,[]) for b in ids},'scope':'owner runtime' if set(ids)&owner_ids else 'compiler proc-macro excluded'})
assert all(x['binary_ids'] and all(x['resolved_objects'].values()) for x in profiles),[(x['path'],x['binary_ids']) for x in profiles if not x['binary_ids'] or not all(x['resolved_objects'].values())]
(r/'final-profraw-list').write_text('\n'.join(map(str,raw))+'\n')
runtime_raw=[x['path'] for x in profiles if x['scope']=='owner runtime']
profile=r/'final.profdata';subprocess.run([str(tool/'llvm-profdata'),'merge','-sparse',*runtime_raw,'-o',str(profile)],check=True)
base=[str(tool/'llvm-cov'),'export',str(binaries[0]),'-instr-profile='+str(profile)]
for p in binaries[1:]:base+=['-object',str(p)]
base+=files
with (r/'final-owners-coverage.json').open('w') as f, (r/'final-coverage.stderr').open('w') as err:subprocess.run(base,stdout=f,stderr=err,check=True)
show=base.copy();show[1]='show'
with (r/'final-owners-coverage.txt').open('w') as f, (r/'final-show.stderr').open('w') as err:subprocess.run(show,stdout=f,stderr=err,check=True)
data=json.loads((r/'final-owners-coverage.json').read_text());summaries={f['filename']:f['summary'] for d in data['data'] for f in d['files']}
(r/'final-coverage-binding.json').write_text(json.dumps({'phases':phases,'command':base,'source_hashes':{p:sha(pathlib.Path(p)) for p in files},'objects':objects,'profiles':profiles,'compiler_objects':compiler_objects,'merged':{'path':str(profile),'sha256':sha(profile)},'summary':summaries,'raw_count':len(raw),'object_count':len(objects),'multiplicity':'Multiple DSO loads/copies and allocator child legitimately emit several profiles for the same object; each raw binary ID is independently resolved, counts need not match.'},indent=2)+'\n')
print('objects',len(objects),'raws',len(raw));print('stderr', (r/'final-coverage.stderr').read_text());print(json.dumps({k:{j:v[j] for j in ('lines','functions','regions')} for k,v in summaries.items()},indent=2))
