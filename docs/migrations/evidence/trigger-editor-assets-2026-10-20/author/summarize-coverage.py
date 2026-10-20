import hashlib,json
from pathlib import Path
p=Path(__file__).parent
text=(p/'07-final-coverage.lcov').read_text()
records=[]
for block in text.split('end_of_record'):
 lines=block.strip().splitlines()
 if not lines: continue
 source=next(x[3:] for x in lines if x.startswith('SF:'))
 definitions={}
 hits={}
 executable={}
 for line in lines:
  if line.startswith('FN:'):
   start,name=line[3:].split(',',1);definitions[name]=int(start)
  elif line.startswith('FNDA:'):
   count,name=line[5:].split(',',1);hits[name]=int(count)
  elif line.startswith('DA:'):
   start,count=line[3:].split(',',1);executable[int(start)]=int(count)
 starts={}
 for name,start in definitions.items(): starts[start]=starts.get(start,0)+hits.get(name,0)
 records.append(dict(source=source,source_sha256=hashlib.sha256(Path(source).read_bytes()).hexdigest(),llvm_lh=int(next(x[3:] for x in lines if x.startswith('LH:'))),llvm_lf=int(next(x[3:] for x in lines if x.startswith('LF:'))),emitted_da_records_hit=sum(n>0 for n in executable.values()),emitted_da_records_total=len(executable),source_starts_hit=sum(n>0 for n in starts.values()),source_starts_total=len(starts),raw_instantiations_hit=sum(n>0 for n in hits.values()),raw_instantiations_total=len(definitions),uncovered_lines=[n for n,c in executable.items() if c==0],uncovered_source_starts=[n for n,c in starts.items() if c==0]))
result=dict(tool='cargo-llvm-cov 0.8.7',scope='--lib; owning assets.rs only; no branch/native/browser coverage claim',lcov_sha256=hashlib.sha256((p/'07-final-coverage.lcov').read_bytes()).hexdigest(),records=records)
(p/'coverage-summary.json').write_text(json.dumps(result,indent=2)+'\n')
print(json.dumps(result,indent=2))
