import {test} from 'node:test';
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {GenMapping,addSegment,setSourceContent,toEncodedMap} from '@jridgewell/gen-mapping';
import {convertCapture, mergeReports, requireReceipts} from '../scripts/browser-coverage.mjs';

const path='/fixture/callbacks.jsx';
const source="// 😀\nexport function called() {\n  return 'hit';\n}\nexport function missed() {\n  return 'never';\n}\n";
const hash=text=>createHash('sha256').update(text).digest('hex');
const inventory={runId:'one',root:'/fixture',files:{[path]:{source,sha256:hash(source)},'/fixture/unloaded.jsx':{source:'export default 1;',sha256:hash('export default 1;')}}};
function capture(){
  const generator=new GenMapping();setSourceContent(generator,path,source);
  source.split('\n').forEach((text,line)=>{
    addSegment(generator,line,0,path,line,0);
    if(text.length)addSegment(generator,line,text.length,path,line,text.length);
  });
  const map=toEncodedMap(generator);
  const generated=source+'\n//# sourceMappingURL=data:application/json;base64,'+Buffer.from(JSON.stringify(map)).toString('base64');
  return {runId:'one',entries:[{url:'http://127.0.0.1:8322/callbacks.jsx',source:generated,functions:[
    {functionName:'',isBlockCoverage:true,ranges:[{startOffset:0,endOffset:generated.length,count:1}]},
    {functionName:'called',isBlockCoverage:true,ranges:[{startOffset:source.indexOf('function called'),endOffset:source.indexOf('export function missed'),count:1}]},
    {functionName:'missed',isBlockCoverage:true,ranges:[{startOffset:source.indexOf('function missed'),endOffset:source.length,count:0}]},
  ]}]};
}
test('mapped original line counts retain an uncalled callback and a never-loaded owner',async()=>{
  const result=await convertCapture(inventory,capture());
  assert.ok(result.lines[path][3]>0);
  assert.equal(result.lines[path][6],0);
  assert.deepEqual(result.neverLoaded,['/fixture/unloaded.jsx']);
});
test('complementary captures merge while a line missed by both remains uncovered',async()=>{
  const a=await convertCapture(inventory,capture());
  const b=structuredClone(a);b.lines[path][3]=0;b.lines[path][6]=1;a.lines[path][4]=0;b.lines[path][4]=0;
  const merged=mergeReports(inventory,[a,b]);
  assert.ok(merged.lines[path][3]>0);assert.ok(merged.lines[path][6]>0);assert.equal(merged.lines[path][4],0);
});
test('an exact plain-JavaScript source needs no map',async()=>{
  const simple={runId:'one',root:'/fixture',files:{'/fixture/plain.mjs':{source:'export const a=1;',sha256:hash('export const a=1;')}}};
  const result=await convertCapture(simple,{runId:'one',entries:[{url:'http://localhost/plain.mjs',source:'export const a=1;',functions:[{functionName:'',isBlockCoverage:true,ranges:[{startOffset:0,endOffset:17,count:1}]}]}]});
  assert.equal(result.lines['/fixture/plain.mjs'][1],1);
});
for(const [name,change,diagnostic] of [
  ['foreign run',c=>{c.runId='other';},/coverage foreign run/],
  ['missing map',c=>{c.entries[0].source=source+'// transformed';},/coverage missing source map/],
  ['stale original',c=>{c.entries[0].source=c.entries[0].source.replace(/base64,.+/, 'base64,'+Buffer.from(JSON.stringify({version:3,sources:[path],sourcesContent:['stale'],names:[],mappings:'AAAA'})).toString('base64'));},/coverage original source mismatch/],
  ['empty mappings',c=>{c.entries[0].source=c.entries[0].source.replace(/base64,.+/, 'base64,'+Buffer.from(JSON.stringify({version:3,sources:[path],sourcesContent:[source],names:[],mappings:''})).toString('base64'));},/coverage unusable mappings/],
  ['invalid range',c=>{c.entries[0].functions[0].ranges[0].endOffset=100000;},/coverage invalid V8 range/],
  ['missing functions',c=>{c.entries[0].functions=[];},/coverage missing whole-program evidence/],
  ['truncated root range',c=>{c.entries[0].functions[0].ranges[0].startOffset=1;},/coverage missing whole-program evidence/],
  ...[['invalid original line','AAo4BA'],['invalid source index','ACAA'],['invalid original column','AAAo4B']].map(([name,mappings])=>[name,c=>{
    c.entries[0].source=c.entries[0].source.replace(/base64,.+/,'base64,'+Buffer.from(JSON.stringify({version:3,sources:[path],sourcesContent:[source],names:[],mappings})).toString('base64'));
  },/coverage invalid source mapping/]),
])test(name+' is refused with its own diagnostic',async()=>{
  const c=capture();change(c);
  if(name!=='invalid range'&&c.entries[0].functions[0])c.entries[0].functions[0].ranges[0].endOffset=c.entries[0].source.length;
  await assert.rejects(convertCapture(inventory,c),diagnostic);
});
test('selected tests require a receipt from this run and a successful outcome',()=>{
  const selected=[{id:'a',status:'passed',receipt:{runId:'one',testId:'a'}},{id:'b',status:'skipped'}];
  assert.equal(requireReceipts('one',selected).length,1);
  assert.throws(()=>requireReceipts('one',[{id:'a',status:'passed'}]),/coverage missing test receipt: a/);
  assert.throws(()=>requireReceipts('one',[{id:'a',status:'passed',receipt:{runId:'other'}}]),/coverage foreign run/);
  assert.throws(()=>requireReceipts('one',[{id:'a',status:'failed',receipt:{runId:'one',testId:'a'}}]),/coverage unsuccessful test: a/);
  assert.throws(()=>requireReceipts('one',[{id:'a',status:'passed',receipt:{runId:'one',testId:'b'}}]),/coverage wrong test receipt: a/);
  assert.throws(()=>requireReceipts('one',[{id:'a',status:'skipped'}]),/coverage no completed receipts/);
  assert.throws(()=>requireReceipts('one',[]),/coverage no selected tests/);
});
