import test from 'node:test';
import assert from 'node:assert/strict';
import {NativeEngine} from '../engine/native-engine.mjs';
import {Store} from '../engine/store.mjs';
import {backend,connection,session,yaml} from './native-fixture.mjs';

const capabilities={metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false};
const retained=(text,generation,state)=>{
  const snapshot=session(text,generation,connection(state));
  snapshot.document.parameters.find(parameter=>parameter.id==='drums.level').normalisedValue=.5;
  return snapshot;
};

test('a recreated production consumer follows an admitted document after its reply is lost without preparing it again',{timeout:3000},async t=>{
  const previous=globalThis.window,initial=retained(yaml,7,{}),bridge=backend(initial);
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const original=new NativeEngine({heartbeatMs:0,pollMs:0});let recreated,store,candidate;
  const admitted=Promise.withResolvers(),lostReply=Promise.withResolvers(),following=Promise.withResolvers(),completion=Promise.withResolvers();
  t.after(()=>{completion.resolve({state:'failed',request_id:'1',error:'Test ended'});store?.close();recreated?.close();original.close();globalThis.window=previous;});
  await original.initialize();const before=original.getPatch();
  bridge.functions.prepareInstrumentSessionPatch=(_instance,attachment,request,revision,text)=>{
    assert.equal(attachment,initial.connection.attachmentId);assert.equal(request,'1');assert.equal(revision,'0');candidate=text;
    bridge.setSession(retained(yaml,7,{activeRequestId:'1',nextRequestId:'2'}));admitted.resolve();return lostReply.promise;
  };
  const pending=original.setVoicePolicy({limit:9});await admitted.promise;
  original.close();lostReply.reject(new Error('Lost admission reply'));await assert.rejects(pending,/Lost admission reply/);
  const attachment='9007199254740994';
  bridge.setSession(retained(yaml,7,{attachmentId:attachment,activeRequestId:'1',nextRequestId:'2'}));
  let polls=0;
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    assert.equal(request,'1');polls++;
    if(polls===1)return {request_id:request,state:'running',revision:'0'};
    following.resolve();return completion.promise;
  };
  recreated=new NativeEngine({heartbeatMs:0,pollMs:0});let initialized=false;
  const initialization=recreated.initialize().then(value=>{initialized=true;return value;});
  await Promise.race([following.promise,initialization.then(()=>assert.fail('editor became usable before following the admitted native job'))]);
  assert.equal(initialized,false);assert.equal(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').length,1);
  bridge.setSession(retained(candidate,8,{attachmentId:attachment,revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}}));
  completion.resolve({request_id:'1',state:'completed',revision:'1'});await initialization;
  store=new Store(recreated);
  assert.equal(recreated.getPatch().voicePolicy.limit,9);assert.equal(recreated.getPatch().params['drums.level'].value,.5);
  assert.equal(store.undoCount,1);assert.equal(recreated.connection.attachmentId,attachment);
  assert.equal(recreated.connection.activeRequestId,undefined);assert.deepEqual(original.getPatch(),before,'closed editor never publishes the late accepted document');
  bridge.functions.undoInstrumentSession=(_instance,owner,request,revision)=>{
    assert.equal(owner,attachment);assert.equal(request,'2');assert.equal(revision,'1');return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    assert.equal(request,'2');bridge.setSession(retained(yaml,9,{attachmentId:attachment,revision:'2',nextRequestId:'3',history:{undoCount:0,redoCount:1}}));
    return {request_id:request,state:'completed',revision:'2'};
  };
  await store.undo();assert.equal(store.undoCount,0);assert.equal(store.canRedo,true);assert.equal(recreated.getPatch().voicePolicy.limit,before.voicePolicy.limit);
  assert.equal(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').length,1,'recovery and Undo never resubmit the lost edit');
});

for(const [name,outcome,delta,error] of [
  ['another request',{request_id:'2',state:'completed',revision:'1'},{},/another request/],
  ['unknown state',{request_id:'1',state:'mystery'},{},/Unknown native recovery state/],
  ['another attachment',{request_id:'1',state:'completed',revision:'1'},{attachmentId:'9'},/incoherent/],
  ['another instance',{request_id:'1',state:'completed',revision:'1'},{instanceId:'9'},/incoherent/],
  ['regressed cursor',{request_id:'1',state:'completed',revision:'1'},{nextRequestId:'1'},/incoherent/],
  ['noncanonical revision',{request_id:'1',state:'completed',revision:'01'},{},/incoherent/],
  ['unchanged revision',{request_id:'1',state:'completed',revision:'0'},{},/incoherent/],
  ['future revision',{request_id:'1',state:'completed',revision:'2'},{},/incoherent/],
  ['still active terminal job',{request_id:'1',state:'completed',revision:'1'},{activeRequestId:'1'},/incoherent/],
])test(`recreated consumer rejects ${name} without admitting work`,async t=>{
  const previous=globalThis.window,initial=retained(yaml,7,{activeRequestId:'1',nextRequestId:'2'}),bridge=backend(initial);
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});
  bridge.functions.getInstrumentSessionJobStatus=()=>{bridge.setSession(retained(yaml,8,{revision:'1',nextRequestId:'2',...delta}));return outcome;};
  await assert.rejects(engine.initialize(),error);
  assert.equal(bridge.calls.some(([method])=>['prepareInstrumentSessionPatch','prepareSampler','setParameter'].includes(method)),false);
});

test('a continuing stream of other admitted work cannot make startup recovery unbounded',async t=>{
  const previous=globalThis.window,initial=retained(yaml,7,{activeRequestId:'1',nextRequestId:'2'}),bridge=backend(initial);let request=1;
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});
  bridge.functions.getInstrumentSessionJobStatus=id=>{
    assert.equal(id,String(request));const revision=String(request);request++;
    bridge.setSession({...initial,connection:connection({revision,activeRequestId:String(request),nextRequestId:String(request+1)})});
    return {request_id:id,state:'completed',revision};
  };
  await assert.rejects(engine.initialize(),/timed out/);
  assert.equal(bridge.calls.filter(([method])=>method==='getInstrumentSessionJobStatus').length,2000);
});

for(const recoveryError of ['Selected sample preparation failed',undefined])test(`a failed recovered job with ${recoveryError?'a reason':'no reason'} leaves accepted editing usable and permits a later native request`,async t=>{
  const previous=globalThis.window,initial=retained(yaml,7,{activeRequestId:'1',nextRequestId:'2'}),bridge=backend(initial);
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store,candidate;
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    if(request==='1'){bridge.setSession(retained(yaml,7,{nextRequestId:'2'}));return {request_id:request,state:'failed',error:recoveryError};}
    assert.equal(request,'2');bridge.setSession(retained(candidate,8,{revision:'1',nextRequestId:'3',history:{undoCount:1,redoCount:0}}));
    return {request_id:request,state:'completed',revision:'1'};
  };
  await engine.initialize();store=new Store(engine);
  assert.equal(store.getSnapshot().phase,'failed');assert.equal(store.getSnapshot().error,recoveryError??'Native preparation failed');
  assert.equal(store.getSnapshot().busy,false);assert.equal(store.undoCount,0);
  assert.equal(engine.getPatch().name,'Advanced Drum Kit');assert.equal(engine.getPatch().params['drums.level'].value,.5);
  bridge.functions.prepareInstrumentSessionPatch=(_instance,_attachment,request,revision,text)=>{
    assert.equal(request,'2');assert.equal(revision,'0');candidate=text;return {status:'accepted',job_id:request};
  };
  await store.operation('Rename','renameNode','root','After failed recovery');
  assert.equal(engine.getPatch().name,'After failed recovery');assert.equal(store.undoCount,1);assert.equal(store.getSnapshot().error,null);
  assert.equal(engine.initializationError,null,'a subsequent successful edit clears the startup warning');
  assert.equal(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').length,1);
});

for(const phase of ['negotiation','initial snapshot','status','accepted snapshot'])test(`closing during recovery ${phase} stops all later native calls and publication`,{timeout:2000},async t=>{
  const previous=globalThis.window,initial=retained(yaml,7,{activeRequestId:'1',nextRequestId:'2'}),bridge=backend(initial);
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0}),entered=Promise.withResolvers(),held=Promise.withResolvers();let response,reads=0;
  t.after(()=>{held.resolve(response);engine.close();globalThis.window=previous;});
  const negotiation=bridge.functions.negotiateEditorProtocol;
  if(phase==='negotiation')bridge.functions.negotiateEditorProtocol=()=>{response=negotiation();entered.resolve();return held.promise;};
  bridge.functions.getSamplerSession=()=>{
    reads++;const current=reads===1?initial:retained(yaml,8,{revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}});
    if((phase==='initial snapshot'&&reads===1)||(phase==='accepted snapshot'&&reads===2)){response=current;entered.resolve();return held.promise;}
    return current;
  };
  bridge.functions.getInstrumentSessionJobStatus=()=>{
    const completed={request_id:'1',state:'completed',revision:'1'};
    if(phase==='status'){response=completed;entered.resolve();return held.promise;}return completed;
  };
  const before=engine.getPatch(),initialization=engine.initialize();await entered.promise;
  engine.close();const calls=bridge.calls.length;held.resolve(response);await assert.rejects(initialization,/closed/);
  assert.equal(bridge.calls.length,calls,'a retired initialization cannot issue another native request');
  assert.deepEqual(engine.getPatch(),before,'a retired initialization cannot publish its late snapshot');
});

test('an already closed consumer cannot begin recovery or negotiate another native request',async t=>{
  const previous=globalThis.window,bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});engine.close();
  await assert.rejects(engine.initialize(),/closed/);assert.equal(bridge.calls.length,0);
});

for(const phase of ['window title','feedback'])test(`closing during initialization ${phase} cannot attach later subscriptions`,{timeout:2000},async t=>{
  const previous=globalThis.window,bridge=backend(),entered=Promise.withResolvers(),held=Promise.withResolvers();
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities:{...capabilities,windowTitle:phase==='window title',metering:phase==='feedback'},backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{held.resolve(phase==='feedback'?true:{status:'accepted'});engine.close();globalThis.window=previous;});
  bridge.functions[phase==='feedback'?'subscribeMeter':'setTriggerWindowTitle']=()=>{entered.resolve();return held.promise;};
  const initialization=engine.initialize();await entered.promise;engine.close();const calls=bridge.calls.length;
  held.resolve(phase==='feedback'?true:{status:'accepted'});await assert.rejects(initialization,/closed/);
  assert.equal(bridge.calls.length,calls,'closed feedback cannot issue a later visibility request');
  assert.equal(bridge.events.size,0,'closed initialization cannot register parameter feedback');
});
