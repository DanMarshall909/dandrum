import test from 'node:test';
import assert from 'node:assert/strict';
import {NativeEngine} from '../engine/native-engine.mjs';
import {backend,session,connection} from './native-fixture.mjs';

const digest='2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5';
const file=()=>({resourceId:`selected/${digest}.wav`,name:'distinct-selected.wav',contentSha256:digest,encodedBytes:60,sampleRateHz:32000,channelCount:2,frameCount:4});
const capabilities={metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false};

test('native selection consumes choosing and preparation status and returns truthful admitted-resource file references',async t=>{
  const previous=globalThis.window,bridge=backend(),id='9007199254740993';
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});await engine.initialize();
  const before=engine.getPatch(),phases=['choosing','preparing','ready'];
  bridge.functions.chooseSamplerFiles=()=>({selectionId:id,phase:'choosing'});
  bridge.functions.getSamplerSelectionStatus=request=>{assert.equal(request,id);const phase=phases.shift();assert.ok(phase,'bounded expected polling');return {selectionId:id,phase,...(phase==='ready'?{files:[file()]}:{})};};
  const files=await engine.chooseSamples();
  assert.deepEqual(files,[{instanceId:engine.connection.instanceId,name:'distinct-selected.wav',uri:`selected/${digest}.wav`,resourceId:`selected/${digest}.wav`,selectionId:id,contentSha256:digest,encodedBytes:60,sampleRate:32000,channels:2,frameCount:4,duration:4/32000}]);
  assert.deepEqual(engine.getPatch(),before,'selection alone cannot publish an instrument edit');
  assert.deepEqual(phases,[]);assert.equal(bridge.calls.filter(([method])=>method==='chooseSamplerFiles').length,1);
});


async function setup(t){
  const previous=globalThis.window,bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});await engine.initialize();
  bridge.functions.chooseSamplerFiles=()=>({selectionId:'1',phase:'choosing'});
  bridge.functions.releaseSamplerSelection=id=>({selectionId:id,phase:'released'});
  return {engine,bridge};
}
for(const phase of ['cancelled','failed'])test(`native ${phase} selection releases its terminal outcome without editing the patch`,async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch();
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase,...(phase==='failed'?{error:'unsupportedEncoding'}:{})});
  if(phase==='cancelled')assert.deepEqual(await engine.chooseSamples(),[]);
  else await assert.rejects(engine.chooseSamples(),error=>error.code==='unsupportedEncoding'&&error.message.includes('unsupportedEncoding'));
  assert.deepEqual(engine.getPatch(),before);
  assert.deepEqual(bridge.calls.filter(([method])=>method==='releaseSamplerSelection'),[['releaseSamplerSelection','1']]);
});

const invalidFiles=[
  ['external path',f=>{f.resourceId='/tmp/asserted.wav';}],
  ['mismatched digest',f=>{f.contentSha256='1'.repeat(64);}],
  ['invalid digest',f=>{f.contentSha256='A'.repeat(64);f.resourceId=`selected/${f.contentSha256}.wav`;}],
  ['display name bytes',f=>{f.name='🥁'.repeat(65);}],
  ['empty display name',f=>{f.name='';}],
  ['encoded limit',f=>{f.encodedBytes=67108865;}],
  ['encoded type',f=>{f.encodedBytes='60';}],
  ['rate',f=>{f.sampleRateHz=0;}],
  ['rate limit',f=>{f.sampleRateHz=4294967296;}],
  ['channels',f=>{f.channelCount=3;}],
  ['frame type',f=>{f.frameCount='4';}],
  ['frames',f=>{f.frameCount=0;}],
  ['frame limit',f=>{f.frameCount=33554433;}],
  ['impossible payload',f=>{f.frameCount=100;}],
];
for(const [name,mutate] of invalidFiles)test(`native ready selection rejects ${name} and releases the known terminal lease`,async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch(),bad=file();mutate(bad);
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'ready',files:[bad]});
  await assert.rejects(engine.chooseSamples(),/Invalid native sample selection/);
  assert.deepEqual(engine.getPatch(),before);assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,1);
});
for(const count of [0,17])test(`native ready selection rejects ${count} files`,async t=>{
  const {engine,bridge}=await setup(t);bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'ready',files:Array.from({length:count},file)});
  await assert.rejects(engine.chooseSamples(),/Invalid native sample selection/);
});
test('native selected metadata deduplicates byte accounting while rejecting inconsistent owners or excess unique content',async t=>{
  const {engine,bridge}=await setup(t),large={...file(),encodedBytes:67108864};
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'ready',files:Array.from({length:5},()=>({...large,uri:'/tmp/untrusted.wav',duration:999}))});
  const files=await engine.chooseSamples();assert.equal(files.length,5);assert.ok(files.every(f=>f.uri===large.resourceId&&f.duration===4/32000));
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'ready',files:[file(),{...file(),frameCount:3}]});
  await assert.rejects(engine.chooseSamples(),/Invalid native sample selection/);
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'ready',files:Array.from({length:5},(_,i)=>{const hash=String(i+1).repeat(64);return {...large,contentSha256:hash,resourceId:`selected/${hash}.wav`};})});
  await assert.rejects(engine.chooseSamples(),/Invalid native sample selection/);
});
for(const [name,started,status] of [
  ['invalid admission id',{selectionId:'0',phase:'choosing'},null],
  ['overflow admission id',{selectionId:'18446744073709551616',phase:'choosing'},null],
  ['invalid admission phase',{selectionId:'1',phase:'ready'},null],
  ['foreign status identity',null,{selectionId:'2',phase:'ready',files:[file()]}],
  ['unknown phase',null,{selectionId:'1',phase:'done'}],
])test(`native selection rejects ${name} without releasing unknown ownership`,async t=>{
  const {engine,bridge}=await setup(t);
  if(started)bridge.functions.chooseSamplerFiles=()=>started;
  bridge.functions.getSamplerSelectionStatus=()=>{assert.ok(status,'invalid admission cannot query status');return status;};
  await assert.rejects(engine.chooseSamples(),/Invalid native sample selection/);
  assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,0);
});
for(const retirement of ['close','replacement'])test(`native selection ignores a late ready result after ${retirement}`,async t=>{
  const {engine,bridge}=await setup(t),entered=Promise.withResolvers(),completion=Promise.withResolvers();
  t.after(()=>completion.resolve({selectionId:'1',phase:'cancelled'}));
  bridge.functions.getSamplerSelectionStatus=()=>{entered.resolve();return completion.promise;};
  const pending=engine.chooseSamples();await Promise.race([entered.promise,pending.then(()=>assert.fail('selection completed before native status'))]);
  if(retirement==='close')engine.close();
  else{bridge.setSession(session(undefined,8,connection({instanceId:'42',attachmentId:'43'})));await engine.requestGeneration(8);}
  const before=engine.getPatch();completion.resolve({selectionId:'1',phase:'ready',files:[file()]});
  await assert.rejects(pending,/closed|stale/i);assert.deepEqual(engine.getPatch(),before);
  assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,0,'no new access through retired ownership');
});
test('closing a native chooser before admission resolves prevents status access',async t=>{
  const {engine,bridge}=await setup(t),completion=Promise.withResolvers();t.after(()=>completion.resolve({selectionId:'1',phase:'choosing'}));
  bridge.functions.chooseSamplerFiles=()=>completion.promise;
  const pending=engine.chooseSamples();engine.close();completion.resolve({selectionId:'1',phase:'choosing'});
  await assert.rejects(pending,/closed/i);assert.equal(bridge.calls.filter(([method])=>method==='getSamplerSelectionStatus').length,0);
});
test('native release failure remains explicit and preserves an earlier selection error',async t=>{
  const {engine,bridge}=await setup(t),cleanup={code:'releaseUnavailable'};
  bridge.functions.releaseSamplerSelection=()=>Promise.reject(cleanup);
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'cancelled'});
  await assert.rejects(engine.chooseSamples(),error=>error===cleanup);
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'failed',error:'unsupportedEncoding'});
  await assert.rejects(engine.chooseSamples(),error=>error.cause?.code==='unsupportedEncoding'&&error.message.includes('unsupportedEncoding')&&error.message.includes('releaseUnavailable'));
});
