import test from 'node:test';
import assert from 'node:assert/strict';
import {parse,stringify} from 'yaml';
import {NativeEngine} from '../engine/native-engine.mjs';
import {Store} from '../engine/store.mjs';
import {entityDelta} from '../engine/entity-delta.mjs';
import {backend,session,connection,yaml} from './native-fixture.mjs';

const capabilities={metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false};

test('CLAP document edits publish once through native history and undo redo preserve current host values',async t=>{
  const previous=globalThis.window,start=9007199254740993n;
  const snapshot=(text,generation,revision,offset,undoCount,redoCount,value)=>{
    const state=session(text,generation,connection({revision:String(revision),nextRequestId:String(start+BigInt(offset)),history:{undoCount,redoCount}}));
    state.document.parameters.find(parameter=>parameter.id==='drums.level').normalisedValue=value;return state;
  };
  const initial=snapshot(yaml,7,0,0,0,0,.37),bridge=backend(initial);
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0}),entered=Promise.withResolvers(),completion=Promise.withResolvers();let store,candidate,action;
  t.after(()=>{completion.resolve({state:'failed',error:'Test ended'});store?.close();engine.close();globalThis.window=previous;});
  await engine.initialize();store=new Store(engine);
  for(const method of ['prepareSampler','getUiJobStatus'])bridge.functions[method]=()=>assert.fail('CLAP document edits must use the persistent native session protocol');
  const expectOwner=(instance,attachment,request,revision,offset)=>{
    assert.equal(instance,initial.connection.instanceId);assert.equal(attachment,initial.connection.attachmentId);
    assert.equal(request,String(start+BigInt(offset)));assert.equal(revision,String(offset));
  };
  bridge.functions.prepareInstrumentSessionPatch=(instance,attachment,request,revision,text)=>{
    expectOwner(instance,attachment,request,revision,0);assert.equal(parse(text).metadata.name,'Retained native name');candidate=text;action='edit';return {status:'accepted',job_id:request};
  };
  bridge.functions.undoInstrumentSession=(instance,attachment,request,revision)=>{expectOwner(instance,attachment,request,revision,1);action='undo';return {status:'accepted',job_id:request};};
  bridge.functions.redoInstrumentSession=(instance,attachment,request,revision)=>{expectOwner(instance,attachment,request,revision,2);action='redo';return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    if(action==='edit'){entered.resolve();return completion.promise;}
    assert.ok(action==='undo'||action==='redo');
    const undo=action==='undo',revision=undo?2:3;
    bridge.setSession(snapshot(undo?yaml:candidate,undo?9:10,revision,revision,undo?0:1,undo?1:0,.61));
    return {state:'completed',request_id:request,revision:String(revision)};
  };
  const pending=store.operation('Rename instrument','renameNode','root','Retained native name');
  await Promise.race([entered.promise,pending.then(()=>assert.fail('edit published before native completion'))]);
  assert.equal(engine.getPatch().name,'Advanced Drum Kit');assert.equal(store.undoCount,0);
  bridge.setSession(snapshot(candidate,8,1,1,1,0,.61));
  completion.resolve({state:'completed',request_id:String(start),revision:'1'});await pending;
  assert.equal(engine.getPatch().name,'Retained native name');assert.equal(engine.getPatch().params['drums.level'].value,.61);
  assert.equal(store.undoCount,1,'one edit has one native history entry');
  assert.equal(store.canUndo,true);await store.undo();
  assert.equal(engine.getPatch().name,'Advanced Drum Kit');assert.equal(engine.getPatch().params['drums.level'].value,.61);
  assert.equal(store.undoCount,0);assert.equal(store.canRedo,true,'native redo is usable');assert.equal(store.redoLabel,'Edit instrument');
  await store.redo();
  assert.equal(engine.getPatch().name,'Retained native name');assert.equal(engine.getPatch().params['drums.level'].value,.61);
  assert.equal(store.undoCount,1);assert.equal(store.canRedo,false);await engine.redoSession();
  assert.deepEqual(bridge.calls.filter(([method])=>['prepareInstrumentSessionPatch','undoInstrumentSession','redoInstrumentSession'].includes(method)).map(([method])=>method),['prepareInstrumentSessionPatch','undoInstrumentSession','redoInstrumentSession']);
  assert.equal(bridge.calls.filter(([method])=>method==='getInstrumentSessionJobStatus').length,3);
});


for(const outcome of ['rejected','failed'])test(`native document ${outcome} retains its correct admission cursor and the accepted patch`,async t=>{
  const previous=globalThis.window,bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store,candidate;
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  const before=engine.getPatch(),requests=[],rejected={code:'sessionWorkerBusy'};
  bridge.functions.prepareInstrumentSessionPatch=(_instance,_attachment,request,revision,text)=>{
    requests.push(request);assert.equal(revision,'0');candidate=text;
    if(requests.length===1&&outcome==='rejected')return Promise.reject(rejected);
    return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    if(request==='1'&&outcome==='failed'){
      bridge.setSession(session(yaml,7,connection({nextRequestId:'2'})));return {state:'failed',request_id:request,revision:'0',error:'preparationFailed'};
    }
    const expected=outcome==='failed'?'2':'1';assert.equal(request,expected);
    bridge.setSession(session(candidate,8,connection({revision:'1',nextRequestId:String(BigInt(expected)+1n),history:{undoCount:1,redoCount:0}})));
    return {state:'completed',request_id:request,revision:'1'};
  };
  await assert.rejects(store.operation('Rename','renameNode','root','Rejected draft'),error=>outcome==='rejected'?error===rejected:error.message==='preparationFailed');
  assert.deepEqual(engine.getPatch(),before);assert.equal(store.undoCount,0);
  assert.equal(engine.connection.nextRequestId,outcome==='failed'?'2':'1','only admitted failure consumes its native ID');
  await store.operation('Rename','renameNode','root','Accepted retry');
  assert.equal(engine.getPatch().name,'Accepted retry');assert.equal(store.undoCount,1);assert.deepEqual(requests,['1',outcome==='failed'?'2':'1']);
});

for(const action of ['document','undo','redo'])test(`closing an admitted ${action} prevents late status from querying or publishing another snapshot`,async t=>{
  const state=action==='document'?session():session(yaml,7,connection({revision:'2',nextRequestId:'3',history:{undoCount:action==='undo'?1:0,redoCount:action==='redo'?1:0}}));
  const previous=globalThis.window,bridge=backend(state);globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0}),entered=Promise.withResolvers(),completion=Promise.withResolvers();
  t.after(()=>{completion.resolve({state:'failed',error:'Test ended'});engine.close();globalThis.window=previous;});await engine.initialize();
  const before=engine.getPatch(),events=[];engine.on('patch',event=>events.push(event));
  let requestId;
  const method=action==='document'?'prepareInstrumentSessionPatch':action==='undo'?'undoInstrumentSession':'redoInstrumentSession';
  bridge.functions[method]=(_instance,_attachment,request)=>{requestId=request;return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=()=>{entered.resolve();return completion.promise;};
  const pending=action==='document'?engine.renameNode('root','Closed draft'):action==='undo'?engine.undoSession():engine.redoSession();
  await Promise.race([entered.promise,pending.then(()=>assert.fail('edit completed before native status'))]);
  engine.close();const queries=bridge.calls.filter(([method])=>method==='getSamplerSession').length;
  bridge.functions.getSamplerSession=()=>assert.fail('snapshot queried after editor closure');
  completion.resolve({state:'completed',request_id:requestId,revision:action==='document'?'1':'3'});
  await assert.rejects(pending,/Engine is closed/);
  assert.equal(bridge.calls.filter(([method])=>method==='getSamplerSession').length,queries);
  assert.deepEqual(engine.getPatch(),before);assert.deepEqual(events.map(event=>event.phase),['preparing']);
});


test('failed snapshot recovery retains the native error code and the prior accepted patch',async t=>{
  const previous=globalThis.window,bridge=backend(),failure={code:'sessionWorkerBusy'};
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});await engine.initialize();
  const before=engine.getPatch();
  bridge.functions.prepareInstrumentSessionPatch=()=>Promise.reject(failure);
  bridge.functions.getSamplerSession=()=>{throw new Error('Snapshot unavailable');};
  await assert.rejects(engine.renameNode('root','Rejected'),error=>error.cause===failure&&error.message.includes('sessionWorkerBusy')&&error.message.includes('Snapshot unavailable'));
  assert.deepEqual(engine.getPatch(),before);
});


test('a fresh editor exposes retained native Redo without recreating browser undo closures',async t=>{
  const previous=globalThis.window,state=session(yaml,9,connection({revision:'2',nextRequestId:'3',history:{undoCount:0,redoCount:1}}));
  state.document.parameters.find(parameter=>parameter.id==='drums.level').normalisedValue=.37;
  const bridge=backend(state);globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store;
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  bridge.functions.redoInstrumentSession=(instance,attachment,request,revision)=>{
    assert.equal(instance,state.connection.instanceId);assert.equal(attachment,state.connection.attachmentId);assert.equal(request,'3');assert.equal(revision,'2');return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const next=session(yaml.replace('Advanced Drum Kit','Restored name'),10,connection({revision:'3',nextRequestId:'4',history:{undoCount:1,redoCount:0}}));
    next.document.parameters.find(parameter=>parameter.id==='drums.level').normalisedValue=.37;bridge.setSession(next);
    return {state:'completed',request_id:request,revision:'3'};
  };
  assert.equal(store.canUndo,false);assert.equal(store.canRedo,true);assert.equal(store.redoLabel,'Edit instrument');
  await store.redo();assert.equal(engine.getPatch().name,'Restored name');assert.equal(engine.getPatch().params['drums.level'].value,.37);
  assert.equal(store.undoCount,1);assert.equal(store.canRedo,false);
  assert.equal(bridge.calls.filter(([method])=>method==='redoInstrumentSession').length,1);
  assert.equal(bridge.calls.some(([method])=>['prepareSampler','prepareInstrumentSessionPatch'].includes(method)),false,'Redo recovers native authority rather than replaying a browser edit');
});


test('an existing rule keeps its canonical editor identity after document and typed mapping commands',async t=>{
  const previous=globalThis.window,bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store,document,step=0;
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  bridge.functions.prepareInstrumentSessionPatch=(_instance,_attachment,request,_revision,text)=>{assert.equal(request,'1');document=text;step=1;return {status:'accepted',job_id:request};};
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request,revision,map,zone,low,high)=>{
    assert.equal(request,'2');assert.equal(revision,'1');assert.equal(map,'kit');assert.equal(zone,'kick');
    const graph=parse(document);graph.assets.sample_maps[0].zones.find(item=>item.id==='kick').key_range=[low,high];document=stringify(graph);step=2;return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    bridge.setSession(session(document,7+step,connection({revision:String(step),nextRequestId:String(step+1),history:{undoCount:step,redoCount:0}})));
    return {state:'completed',request_id:request,revision:String(step)};
  };
  const rules=engine.getPatch().rules.map(rule=>rule.id==='kit.kick'?{...rule,noteLo:70,noteHi:70}:rule);
  await store.operation('Move mapping document','setRules',rules);
  assert.ok(engine.getPatch().rules.some(rule=>rule.id==='kit.kick'));
  await store.operation('Move mapping','updateRule','kit.kick',{noteLo:71,noteHi:71});
  const retained=engine.getPatch().rules.find(rule=>rule.id==='kit.kick');
  assert.ok(retained,'typed mapping preserves the document editor identity');assert.equal(retained.noteLo,71);assert.equal(store.undoCount,2);
});


for(const order of ['document-first','parameter-first'])test(`native document and local parameter redo retain ${order} ordering`,async t=>{
  const previous=globalThis.window,state=session();state.document.parameters.find(parameter=>parameter.id==='drums.level').normalisedValue=.37;
  const bridge=backend(state);globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store,document,action,value=.37,tracing=false;const trace=[];
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  const set=bridge.functions.setParameter;bridge.functions.setParameter=(id,next)=>{value=next;if(tracing)trace.push('parameter');return set(id,next);};
  bridge.functions.prepareInstrumentSessionPatch=(_instance,_attachment,request,_revision,text)=>{document=text;action='edit';return {status:'accepted',job_id:request};};
  bridge.functions.undoInstrumentSession=(_instance,_attachment,request)=>{action='undo';return {status:'accepted',job_id:request};};
  bridge.functions.redoInstrumentSession=(_instance,_attachment,request)=>{action='redo';if(tracing)trace.push('document');return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const revision=action==='edit'?1:action==='undo'?2:3,undo=action==='undo';
    const next=session(undo?yaml:document,7+revision,connection({revision:String(revision),nextRequestId:String(revision+1),history:{undoCount:undo?0:1,redoCount:undo?1:0}}));
    next.document.parameters.find(parameter=>parameter.id==='drums.level').normalisedValue=value;bridge.setSession(next);
    return {state:'completed',request_id:request,revision:String(revision)};
  };
  const documentEdit=()=>store.operation('Rename','renameNode','root','Mixed native name'),parameterEdit=()=>store.changeParam('drums.level',.8);
  if(order==='document-first'){await documentEdit();await parameterEdit();}else{await parameterEdit();await documentEdit();}
  assert.equal(store.undoCount,2);await store.undo();await store.undo();
  assert.equal(engine.getPatch().name,'Advanced Drum Kit');assert.equal(engine.getPatch().params['drums.level'].value,.37);
  tracing=true;await store.redo();await store.redo();
  assert.deepEqual(trace,order==='document-first'?['document','parameter']:['parameter','document']);
  assert.equal(engine.getPatch().name,'Mixed native name');assert.equal(engine.getPatch().params['drums.level'].value,.8);assert.equal(value,.8);
  assert.equal(store.undoCount,2);assert.equal(store.canRedo,false);
});


for(const action of ['document','undo','redo'])test(`closing during ${action} error recovery ignores the late snapshot`,async t=>{
  const state=action==='document'?session():session(yaml,7,connection({revision:'2',nextRequestId:'3',history:{undoCount:action==='undo'?1:0,redoCount:action==='redo'?1:0}}));
  const previous=globalThis.window,bridge=backend(state),failure={code:'sessionWorkerBusy'};
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0}),entered=Promise.withResolvers(),completion=Promise.withResolvers();
  t.after(()=>{completion.resolve(state);engine.close();globalThis.window=previous;});await engine.initialize();
  const before=engine.getPatch(),events=[];engine.on('patch',event=>events.push(event));
  const method=action==='document'?'prepareInstrumentSessionPatch':action==='undo'?'undoInstrumentSession':'redoInstrumentSession';
  bridge.functions[method]=()=>Promise.reject(failure);
  bridge.functions.getSamplerSession=()=>{entered.resolve();return completion.promise;};
  const pending=action==='document'?engine.renameNode('root','Rejected draft'):action==='undo'?engine.undoSession():engine.redoSession();
  await Promise.race([entered.promise,pending.then(()=>assert.fail('edit finished before recovery'),()=>assert.fail('edit rejected before recovery'))]);
  engine.close();
  const late=session(yaml.replace('Advanced Drum Kit','Late native patch'),8,state.connection);
  completion.resolve(late);
  await assert.rejects(pending,error=>error===failure);
  assert.deepEqual(engine.getPatch(),before,'closed editor ignores the recovered native document');
  assert.deepEqual(events.map(event=>event.phase),['preparing']);
});


const unrepresentableEdits=[
  ['renameNode',p=>['kit','Percussion']],
  ['addNode',p=>['root','group']],
  ['moveNode',p=>[p.nodes.find(n=>n.kind==='sound').id,'root',0]],
  ['setRegion',p=>[p.regions[0].id,{name:'Named hit'}]],
  ['setOutput',p=>['kit','main']],
  ['setBus',p=>['main',{channels:[7,8]}]],
  ['setSelector',p=>['kit.hard_snare',{reset:'never'}]],
  ['setSelector',p=>['kit.hard_snare',{policy:'random'}]],
  ['setCandidate',p=>['kit.hard_snare',p.selectors['kit.hard_snare'].candidates[0].id,{name:'Named candidate'}]],
  ['setCandidate',p=>['kit.hard_snare',p.selectors['kit.hard_snare'].candidates[0].id,{muted:true}]],
  ['updateRule',p=>[p.rules[0].id,{name:'Named mapping',noteLo:70,noteHi:70}]],
  ['setRules',p=>[p.rules.map((r,i)=>i? r:{...r,name:'Named mapping',noteLo:70,noteHi:70})]],
  ['setRules',p=>[[...p.rules,{...p.rules[0],id:'new-rule',noteLo:70,noteHi:70}]]],
  ['addHistory',p=>['Trim',{start:.01,end:.5},p.regions[0].id]],
  ['setSlices',p=>[p.assets[0].id,[0,.5],{startNote:70}]],
  ['importAssets',p=>[[{name:'kit.wav',uri:p.assets[0].uri,duration:1,sampleRate:48000,channels:2}]]],
  ['importSamples',p=>[[{name:'kit.wav',uri:p.assets[0].uri,duration:1,sampleRate:48000,channels:2}],{policy:'sequential',startNote:70}]],
  ['mapAssets',p=>[[p.assets[0].id],{policy:'sequential',startNote:70}]],
  ['applyDelta',p=>{const next=structuredClone(p);next.nodes.find(n=>n.id==='kit').name='Delta rename';return [entityDelta(p,next)];}],
];
for(const [index,[method,args]] of unrepresentableEdits.entries())test(`native history refuses unrepresentable edit ${index+1}: ${method}`,async t=>{
  const previous=globalThis.window,bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store,admissions=0;
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  const before=engine.getPatch();
  bridge.functions.prepareInstrumentSessionPatch=(_i,_a,request,_r,text)=>{admissions++;bridge.setSession(session(text,8,connection({revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}})));return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>({state:'completed',request_id:request,revision:'1'});
  const pending=method==='applyDelta'?engine.applyDelta(...args(before)):store.operation('Unsupported edit',method,...args(before));
  await assert.rejects(pending,/unsupported/i);
  assert.equal(admissions,0,'unrepresentable state cannot consume native admission or history');
  assert.deepEqual(engine.getPatch(),before);assert.equal(store.undoCount,0);
});

test('native region DSP edits publish only reconstructible state through Undo Redo and recreation',async t=>{
  const previous=globalThis.window,bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store,candidate,action,recreated;
  t.after(()=>{recreated?.close();store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  const before=engine.getPatch();
  bridge.functions.prepareInstrumentSessionPatch=(_i,_a,request,_r,text)=>{candidate=text;action='edit';return {status:'accepted',job_id:request};};
  bridge.functions.undoInstrumentSession=(_i,_a,request)=>{action='undo';return {status:'accepted',job_id:request};};
  bridge.functions.redoInstrumentSession=(_i,_a,request)=>{action='redo';return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const undo=action==='undo',revision=action==='edit'?1:undo?2:3;
    bridge.setSession(session(undo?yaml:candidate,7+revision,connection({revision:String(revision),nextRequestId:String(revision+1),history:{undoCount:undo?0:1,redoCount:undo?1:0}})));
    return {state:'completed',request_id:request,revision:String(revision)};
  };
  await store.operation('Trim','setRegion','drums.kick',{start:.01,end:.1,reverse:true,root:36,gain:-6,pan:.25,fades:{in:.001,out:.002},loop:{mode:'forward',start:.02,end:.08,crossfade:.0005}});
  const accepted=engine.getPatch();assert.deepEqual(accepted.history,[],'native operation history is the supported Undo authority; no transient processing stack');
  assert.equal(accepted.regions.find(r=>r.id==='drums.kick').start,.01);assert.equal(accepted.regions.find(r=>r.id==='drums.kick').reverse,true);
  const region=accepted.regions.find(r=>r.id==='drums.kick');
  assert.deepEqual([region.root,region.gain,region.pan,region.fades,region.loop],[36,-6,.25,{in:.001,out:.002},{mode:'forward',start:.02,end:.08,crossfade:.0005}]);
  for(const [method,args] of unrepresentableEdits){
    const pending=method==='applyDelta'?engine.applyDelta(...args(accepted)):store.operation('Rejected edit',method,...args(accepted));
    await assert.rejects(pending,/unsupported/i);assert.deepEqual(engine.getPatch(),accepted);assert.equal(store.undoCount,1);
  }
  await store.undo();assert.deepEqual(engine.getPatch(),before);
  await store.redo();assert.deepEqual(engine.getPatch(),accepted);
  engine.close();recreated=new NativeEngine({heartbeatMs:0,pollMs:0});await recreated.initialize();assert.deepEqual(recreated.getPatch(),accepted);
});


for(const action of ['candidate controls','candidate order','candidate removal','voice allocation'])test(`supported native ${action} survives history and reconstruction`,async t=>{
  const previous=globalThis.window,bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store,candidate,phase,recreated;
  t.after(()=>{recreated?.close();store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  const before=engine.getPatch(),selector='kit.hard_snare',ids=before.selectors[selector].candidates.map(c=>c.id);
  bridge.functions.prepareInstrumentSessionPatch=(_i,_a,request,_r,text)=>{candidate=text;phase='edit';return {status:'accepted',job_id:request};};
  bridge.functions.undoInstrumentSession=(_i,_a,request)=>{phase='undo';return {status:'accepted',job_id:request};};
  bridge.functions.redoInstrumentSession=(_i,_a,request)=>{phase='redo';return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const undo=phase==='undo',revision=phase==='edit'?1:undo?2:3;
    bridge.setSession(session(undo?yaml:candidate,7+revision,connection({revision:String(revision),nextRequestId:String(revision+1),history:{undoCount:undo?0:1,redoCount:undo?1:0}})));
    return {state:'completed',request_id:request,revision:String(revision)};
  };
  if(action==='candidate controls')await store.operation('Candidate','setCandidate',selector,ids[0],{weight:2.5,gain:-3,pan:-.2,tune:4});
  else if(action==='candidate order')await store.operation('Order','reorderCandidates',selector,[...ids].reverse());
  else if(action==='candidate removal')await store.operation('Remove','removeCandidate',selector,ids[1]);
  else await store.operation('Voices','setVoicePolicy',{limit:9,steal:'quietest'});
  const accepted=engine.getPatch(),candidates=accepted.selectors[selector].candidates;
  if(action==='candidate controls')assert.deepEqual([candidates[0].weight,candidates[0].gain,candidates[0].pan,candidates[0].tune],[2.5,-3,-.2,4]);
  else if(action==='candidate order')assert.deepEqual(candidates.map(c=>c.id),[...ids].reverse());
  else if(action==='candidate removal')assert.deepEqual(candidates.map(c=>c.id),[ids[0]]);
  else assert.deepEqual([accepted.voicePolicy.limit,accepted.voicePolicy.steal],[9,'quietest']);
  await store.undo();assert.deepEqual(engine.getPatch(),before);
  await store.redo();assert.deepEqual(engine.getPatch(),accepted);
  engine.close();recreated=new NativeEngine({heartbeatMs:0,pollMs:0});await recreated.initialize();assert.deepEqual(recreated.getPatch(),accepted);
});
