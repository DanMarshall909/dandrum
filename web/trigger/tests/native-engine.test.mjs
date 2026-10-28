import test from 'node:test';
import assert from 'node:assert/strict';
import {parse,stringify} from 'yaml';
import {session,connection,backend,yaml} from './native-fixture.mjs';
import {NativeEngine} from '../engine/native-engine.mjs';
import {createWebViewTransport} from '../engine/native-webview-transport.mjs';
import {entityDelta} from '../engine/entity-delta.mjs';
import {Store} from '../engine/store.mjs';
const setup=async t=>{const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});await engine.initialize();t.after(()=>engine.close());return {engine,bridge};};

test('native command failures preserve their useful message through the webview transport',async t=>{
  const failures=[
    ['code-only native rejection',{kind:'reply',error:{code:'capabilityUnavailable'}},'capabilityUnavailable'],
    ['native rejection message',{kind:'reply',error:{code:'nativeRejected',message:'Parameter is unavailable'}},'Parameter is unavailable'],
    ['Error thrown while posting',{kind:'throw',error:new Error('IPC unavailable')},'IPC unavailable'],
    ['string thrown while posting',{kind:'throw',error:'IPC unavailable'},'IPC unavailable'],
  ];
  for(const [name,failure,expected] of failures){
    await t.test(name,async subtest=>{
      let transport;
      transport=createWebViewTransport({instanceId:'17',attachmentId:'23',postMessage:raw=>{
        const request=JSON.parse(raw);
        if(request.method==='setParameter'){
          if(failure.kind==='throw')throw failure.error;
          transport.receive(JSON.stringify({version:1,instanceId:'17',attachmentId:'23',requestId:request.requestId,
            status:'rejected',error:failure.error}));
          return;
        }
        const result=request.method==='getSamplerSession'?session():true;
        transport.receive(JSON.stringify({version:1,instanceId:'17',attachmentId:'23',requestId:request.requestId,status:'accepted',result}));
      }});
      const engine=new NativeEngine({backend:transport.descriptor.backend,pollMs:0,heartbeatMs:0});
      subtest.after(()=>{engine.close();transport.dispose();});
      await engine.initialize();
      const events=[];engine.on('patch',event=>events.push(event));
      await assert.rejects(engine.setParam('drums.level',.41),reason=>
        failure.kind==='reply'?reason.code===failure.error.code:reason===failure.error);
      assert.deepEqual(events.filter(event=>event.phase==='failed').map(event=>event.message),[expected]);
    });
  }
});

test('native window title follows accepted patch names, undo and external replacement',async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch();
  const names=()=>bridge.calls.filter(call=>call[0]==='setTriggerWindowTitle').map(call=>call[1]);
  assert.deepEqual(names(),['Advanced Drum Kit']);
  const name='808: "Night" / 東京 #1';
  await engine.renameNode('root',name);assert.equal(names().at(-1),name);
  await engine.applyDelta(entityDelta(engine.getPatch(),before));assert.equal(names().at(-1),before.name);
  const next=session(engine.session.yaml.replace('Advanced Drum Kit','Guest Set'),engine.generation+1);
  bridge.setSession(next);await engine.requestGeneration(next.document.generation);assert.equal(names().at(-1),'Guest Set');
  const accepted=names();bridge.functions.getUiJobStatus=()=>({state:'failed',error:'Rejected patch'});
  await assert.rejects(engine.renameNode('root','Rejected name'),/Rejected patch/);assert.deepEqual(names(),accepted);
});

test('a window title failure does not reject an already accepted patch edit',async t=>{
  const {engine,bridge}=await setup(t),events=[];engine.on('patch',event=>events.push(event));
  bridge.functions.setTriggerWindowTitle=()=> 'Window unavailable';
  await engine.renameNode('root','Accepted name');assert.equal(engine.getPatch().name,'Accepted name');
  assert.ok(events.some(event=>event.message?.includes('Window title update failed')));
});

test('native edits await real publication; failed/stale preparations retain patch and expose failure',async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch(),events=[];engine.on('patch',value=>events.push(value));
  bridge.functions.getUiJobStatus=()=>({state:'failed',generation:7,error:'Unknown audio asset'});
  await assert.rejects(engine.setRegion('drums.kick',{end:.2}),/Unknown audio asset/);assert.deepEqual(engine.getPatch(),before);assert.deepEqual(events.map(e=>e.phase),['preparing','failed']);
  bridge.functions.getUiJobStatus=()=>({state:'stale',generation:8,error:''});
  await assert.rejects(engine.renameNode('root','Stale'),/stale/i);assert.deepEqual(engine.getPatch(),before);
  assert.equal(bridge.calls.filter(c=>c[0]==='prepareSampler').length,2);
});

test('structural commands serialize through host and reversible operands compile real map changes',async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch(),rules=engine.getPatch().rules;
  rules.find(rule=>rule.id==='kit.kick').noteLo=40;rules.find(rule=>rule.id==='kit.kick').noteHi=40;
  await Promise.all([engine.renameNode('root','Renamed'),engine.setRules(rules)]);
  assert.equal(engine.getPatch().name,'Renamed');assert.equal(engine.getPatch().rules.find(r=>r.id==='kit.kick').noteLo,40);
  assert.deepEqual(bridge.calls.filter(c=>c[0]==='prepareSampler').map(c=>c[2]),[7,8]);
  await engine.applyDelta(entityDelta(engine.getPatch(),before));assert.equal(engine.getPatch().name,before.name);
  const graph=parse(bridge.calls.filter(c=>c[0]==='prepareSampler').at(-1)[1]);assert.equal(graph.assets.sample_maps[0].zones.find(z=>z.id==='kick').key_range[0],36);
});

test('live writes and notes carry current generation and use normalized values without phantom controls',async t=>{
  const {engine,bridge}=await setup(t);await engine.setParam('drums.level',.7,'begin');await engine.noteOn(36,127);await engine.noteOff(36);
  assert.ok(bridge.calls.some(c=>JSON.stringify(c)===JSON.stringify(['setParameter','drums.level',.7,7])));
  assert.ok(bridge.calls.some(c=>JSON.stringify(c)===JSON.stringify(['noteOn',36,1,7])));
  assert.ok(bridge.calls.some(c=>JSON.stringify(c)===JSON.stringify(['noteOff',36,7])));
  await assert.rejects(async()=>engine.setParam('Cutoff',.8),/Unknown parameter/);
  bridge.functions.setParameter=()=> 'Rejected stale instrument generation';
  await assert.rejects(engine.setParam('drums.level',.9),/stale/i);assert.equal(engine.getPatch().params['drums.level'].value,.7);
});

for(const onset of ['held','in-flight'])for(const cleanup of ['release','close'])test(`same-attachment mapping undo and generation refresh preserve ${onset} note until ${cleanup}`,async t=>{
  t.mock.timers.enable({apis:['setInterval']});
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:5});
  await engine.initialize();t.after(()=>engine.close());
  let undo=false,acceptOn;
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request)=>({status:'accepted',job_id:request});
  bridge.functions.undoInstrumentSession=(_instance,_attachment,request)=>{undo=true;return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const revision=undo?'2':'1';
    bridge.setSession(session(undo?yaml:yaml.replace('key_range: [36, 36]','key_range: [60, 60]'),undo?9:8,
      connection({revision,nextRequestId:undo?'3':'2',history:{undoCount:undo?0:1,redoCount:undo?1:0}})));
    return {state:'completed',request_id:request,revision};
  };
  if(onset==='in-flight')bridge.functions.noteOn=()=>new Promise(resolve=>{acceptOn=resolve;});
  const pressed=engine.noteOn(36,127);
  if(onset==='held')await pressed;
  await engine.updateRule('kit.kick',{noteLo:60,noteHi:60});
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,60);
  assert.equal(engine.generation,8);t.mock.timers.tick(5);await Promise.resolve();await Promise.resolve();
  await engine.undoSession();
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,36);
  assert.equal(engine.generation,9);t.mock.timers.tick(5);await Promise.resolve();await Promise.resolve();
  bridge.setSession(session(yaml,10,engine.session.connection));
  await bridge.events.get('parameterStateChanged')({generation:10,sequence:0,parameters:[]});
  assert.equal(engine.generation,10);t.mock.timers.tick(5);await Promise.resolve();await Promise.resolve();
  assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOn'),[['noteOn',36,1,7]]);
  assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOff'),[]);
  const released=cleanup==='close'?engine.close():engine.noteOff(36);
  if(onset==='in-flight'){
    assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOff'),[],'release waits for On admission');
    acceptOn({status:'accepted'});
  }
  await pressed;await released;
  assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOff'),[['noteOff',36,7]],'matching release retains press generation');
  assert.equal(bridge.calls.filter(call=>call[0]==='noteHeartbeat').length,3,'heartbeat survives every same-attachment refresh');
  if(cleanup==='release'){
    bridge.functions.noteOn=()=>({status:'accepted'});
    await engine.noteOn(36,127);await engine.noteOff(36);
    assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOn'),[['noteOn',36,1,7],['noteOn',36,1,10]]);
    assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOff'),[['noteOff',36,7],['noteOff',36,10]]);
  }
  engine.close();engine.close();const calls=bridge.calls.length;
  t.mock.timers.tick(100);await Promise.resolve();assert.equal(bridge.calls.length,calls);
  assert.equal(bridge.events.size,0);
});

test('current note errors remain visible after same-attachment generation refresh',async t=>{
  const {engine,bridge}=await setup(t),events=[];engine.on('patch',event=>events.push(event));
  let finishOn;bridge.functions.noteOn=()=>new Promise(resolve=>{finishOn=resolve;});
  const pressed=engine.noteOn(36,127);
  bridge.setSession(session(yaml,8,engine.session.connection));
  await engine.requestGeneration(8);
  finishOn('Current note admission rejected');await pressed;
  assert.deepEqual(events.filter(event=>event.phase==='failed').map(event=>event.message),['Current note admission rejected']);
});

for(const retirement of ['close','instance replacement','attachment replacement'])test(`late note errors after ${retirement} do not publish editor failures`,async t=>{
  const {engine,bridge}=await setup(t),events=[];engine.on('patch',event=>events.push(event));
  let finishOn;bridge.functions.noteOn=()=>new Promise(resolve=>{finishOn=resolve;});
  const pressed=engine.noteOn(36,127);
  if(retirement==='close')engine.close();
  else{
    const field=retirement==='instance replacement'?'instanceId':'attachmentId';
    bridge.setSession(session(yaml,8,connection({...engine.session.connection,[field]:'42'})));
    await engine.requestGeneration(8);
    assert.equal(engine.connection[field],'42');
    await engine.noteOff(36);assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOff'),[]);
  }
  finishOn('Retired note admission rejected');await pressed;
  assert.deepEqual(events.filter(event=>event.phase==='failed'),[]);
});

test('real meter packets are acknowledged, stale data dropped and waveform comes only from native reduction',async t=>{
  const {engine,bridge}=await setup(t),telemetry=[];engine.on('telemetry',value=>telemetry.push(value));
  bridge.functions.getMeterPacket=()=>({generation:7,sequence:'11',display_valid:true,display_peak:[.3,.6]});
  await engine.pollTelemetry();assert.deepEqual(telemetry.at(-1).meters.main,[.3,.6]);assert.ok(bridge.calls.some(c=>c[0]==='ackMeterPacket'&&c[1]==='11'&&c[2]===7));
  bridge.functions.getMeterPacket=()=>({generation:6,sequence:'12',display_valid:true,display_peak:[1,1]});await engine.pollTelemetry();assert.equal(telemetry.length,1);
  assert.deepEqual(Array.from(engine.getPeaks('drums',600)),[]);await engine.getPeaksAsync('drums',600);assert.deepEqual(Array.from(engine.getPeaks('drums',600)),[.75]);
  assert.ok(bridge.calls.some(c=>c[0]==='requestSourceWaveform'&&c[1]==='drums'&&c[4]===7));
});

test('native import preserves actual metadata and commits real file URIs atomically, without mock voice additions',async t=>{
  const {engine,bridge}=await setup(t);const result=await engine.importSamples([{name:'Tom_C2.wav',uri:'/repo/examples/patches/.tools/trigger-assets/123/Tom_C2.wav',duration:1,sampleRate:48000,bits:16,channels:1}],{policy:'sequential',startNote:48});
  assert.equal(result.assets[0].uri,'/repo/examples/patches/.tools/trigger-assets/123/Tom_C2.wav');assert.equal(result.assets[0].bits,16);assert.equal(engine.getPatch().modules.length,1);
  const graph=parse(bridge.calls.filter(c=>c[0]==='prepareSampler').at(-1)[1]);assert.ok(graph.assets.sample_sources.some(s=>s.path==='.tools/trigger-assets/123/Tom_C2.wav'));
  await assert.rejects(engine.importSamples([{name:'fake.wav',uri:'mock://fake.wav'}]),/native|absolute|metadata/i);
  assert.throws(()=>engine.analyse('drums','pitch'),/unsupported/i);assert.throws(()=>engine.audition('drums',{start:0,end:.1}),/unsupported/i);
});

test('native polling waits for completion and close prevents late prepared publication',async t=>{
  const {engine,bridge}=await setup(t),complete=bridge.functions.getUiJobStatus;let count=0;
  bridge.functions.getUiJobStatus=id=>++count===1?{state:'running',generation:7}:complete(id);
  await engine.renameNode('root','Waited');assert.equal(count,2);assert.equal(engine.getPatch().name,'Waited');
  let entered;const started=new Promise(resolve=>{entered=resolve;});
  bridge.functions.getUiJobStatus=()=>{entered();return {state:'running',generation:8};};
  const pending=engine.renameNode('root','Unpublished');await started;engine.close();await assert.rejects(pending,/closed/);assert.equal(engine.getPatch().name,'Waited');
});

test('host automation reconciles actual normalized native controls and ignores retired generations',async t=>{
  const {engine,bridge}=await setup(t);const event=bridge.events.get('parameterStateChanged');
  event({generation:7,sequence:1,parameters:[{id:'drums.level',name:'Level',value:.4}]});assert.equal(engine.getPatch().params['drums.level'].value,.4);
  event({generation:6,sequence:100,parameters:[{id:'drums.level',name:'Level',value:1}]});assert.equal(engine.getPatch().params['drums.level'].value,.4);
  await engine.newPatch();assert.equal(engine.getPatch().rules.length,0);assert.equal(engine.getPatch().assets.length,0);
  await engine.loadPreset('felt-kit');assert.equal(engine.getPatch().name,'Advanced Drum Kit');assert.equal(engine.getPatch().rules.length,5);
  await engine.reload();assert.equal(engine.getPatch().regions.length,6);
});

test('unsupported audio edits reject before native admission and waveform rejections create no fake peaks',async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch();
  for(const action of [()=>engine.setModuleParam('sample','mode','gated'),()=>engine.addModulator('lfo'),()=>engine.setSelector('kit.hard_snare',{policy:'stack'}),()=>engine.setVoicePolicy({glide:1}),()=>engine.addHistory('Normalize',{},'drums.kick')])await assert.rejects(action(),/unsupported/);
  assert.deepEqual(engine.getPatch(),before);assert.equal(bridge.calls.filter(c=>c[0]==='prepareSampler').length,0);
  bridge.functions.getWaveformJobStatus=()=>({state:'stale',generation:8,error:'Retired waveform'});
  await assert.rejects(engine.getPeaksAsync('drums',64),/Retired waveform/);assert.equal(engine.live.cache.size,0);
});

test('native background telemetry and note heartbeat stop cleanly with the editor',async t=>{
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:5});await engine.initialize();t.after(()=>engine.close());
  await engine.noteOn(36,100);await new Promise(resolve=>setTimeout(resolve,65));
  assert.ok(bridge.calls.some(call=>call[0]==='noteHeartbeat'));assert.ok(bridge.calls.some(call=>call[0]==='getMeterPacket'));
  engine.close();assert.equal(bridge.events.size,0);await new Promise(resolve=>setTimeout(resolve,5));
  const calls=bridge.calls.length;await new Promise(resolve=>setTimeout(resolve,15));assert.equal(bridge.calls.length,calls);
});

test('native preparation has a finite poll budget and preserves accepted patch on timeout',async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch();engine.wait=()=>Promise.resolve();
  bridge.functions.getUiJobStatus=()=>({state:'running',generation:7});
  await assert.rejects(engine.renameNode('root','Never ready'),/timed out/);assert.deepEqual(engine.getPatch(),before);
});

test('successful replacement retires local note intent and stale waveform work without false editor failures',async t=>{
  const {engine,bridge}=await setup(t),phases=[];engine.on('patch',value=>phases.push(value.phase));await engine.noteOn(36,100);
  bridge.functions.noteOff=(_note,generation)=>generation===7?'Rejected stale instrument generation':null;
  await engine.renameNode('root','Replacement');await new Promise(resolve=>setTimeout(resolve,0));
  assert.deepEqual(phases,['preparing','ready']);assert.equal(engine.live.notes.isPressed(36),false);
});

test('new empty patch has no phantom controls and importing a staged sample restores an accepted sampler graph',async t=>{
  const {engine,bridge}=await setup(t);await engine.newPatch();assert.deepEqual(engine.getPatch().params,{});
  await engine.importSamples([{name:'Imported.wav',uri:'/repo/examples/patches/.tools/trigger-assets/123/Imported.wav',duration:1,sampleRate:48000,bits:16,channels:1}],{policy:'sequential',startNote:48});
  assert.equal(engine.getPatch().rules.length,1);assert.ok(engine.getPatch().params['drums.level']);
  const graph=parse(bridge.calls.filter(call=>call[0]==='prepareSampler').at(-1)[1]);assert.equal(graph.modules.find(module=>module.type==='sample_map_player').static.sample_map,'kit');
});

test('a newer device generation refreshes native session and controls while retaining editor undo and selection',async t=>{
  const {engine,bridge}=await setup(t),store=new Store(engine);t.after(()=>store.close());
  await store.operation('Rename','renameNode','root','Retained');store.select({zoneSel:'kit.kick',sampleDraft:{id:'drums.kick',start:.01}});
  const next=session(engine.session.yaml,9,engine.session.connection);bridge.setSession(next);
  await bridge.events.get('parameterStateChanged')({generation:9,sequence:0,parameters:[]});
  assert.equal(engine.generation,9);assert.equal(store.undoCount,1);assert.equal(store.getSnapshot().editor.zoneSel,'kit.kick');assert.equal(store.getSnapshot().patch.name,'Retained');
  await engine.setParam('drums.level',.6);assert.deepEqual(bridge.calls.filter(call=>call[0]==='setParameter').at(-1),['setParameter','drums.level',.6,9]);
});

test('external graph replacement retires old history and selections while preserving editor layout',async t=>{
  const {engine,bridge}=await setup(t),store=new Store(engine),events=[];t.after(()=>store.close());engine.on('patch',value=>events.push(value));
  await store.operation('Rename','renameNode','root','Own edit');store.select({layout:'exp',zoneSel:'retired-zone',sampleDraft:{id:'retired-region'},selection:{kind:'node',id:'retired-node'}});
  const next=session(engine.session.yaml.replace('Own edit','External patch'),9);bridge.setSession(next);
  await bridge.events.get('parameterStateChanged')({generation:9,sequence:0,parameters:[]});
  assert.equal(store.getSnapshot().patch.name,'External patch');assert.equal(store.undoCount,0);assert.equal(store.canRedo,false);
  assert.equal(store.getSnapshot().editor.selection,null);assert.equal(store.getSnapshot().editor.sampleDraft,null);assert.equal(store.getSnapshot().editor.zoneSel,engine.getPatch().rules[0].id);
  assert.equal(store.getSnapshot().editor.layout,'exp');assert.equal(events.at(-1).replacement,true);
});

test('own native preparation defers generation events and preserves the newly accepted command history',async t=>{
  const {engine,bridge}=await setup(t),store=new Store(engine),complete=bridge.functions.getUiJobStatus;t.after(()=>store.close());
  bridge.functions.getUiJobStatus=id=>{const result=complete(id);bridge.events.get('parameterStateChanged')({generation:result.generation,sequence:0,parameters:[]});return result;};
  await store.operation('Rename','renameNode','root','Own accepted edit');await new Promise(resolve=>setTimeout(resolve,0));
  assert.equal(engine.getPatch().name,'Own accepted edit');assert.equal(store.undoCount,1);assert.equal(bridge.calls.filter(call=>call[0]==='getSamplerSession').length,2);
  bridge.functions.getUiJobStatus=()=>{
    bridge.setSession(session(engine.session.yaml,9,engine.session.connection));bridge.events.get('parameterStateChanged')({generation:9,sequence:0,parameters:[]});
    return {state:'stale',generation:9,error:'External device generation'};
  };
  await assert.rejects(engine.renameNode('root','Retired edit'),/External device generation/);await engine.reconciliation;
  assert.equal(engine.generation,9);assert.equal(engine.getPatch().name,'Own accepted edit');assert.equal(store.undoCount,1);
});

test('native initialization retries one rejected meter subscription and refuses to claim readiness if both fail',async t=>{
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});t.after(()=>engine.close());let attempts=0;
  bridge.functions.subscribeMeter=()=>++attempts===2;await engine.initialize();assert.equal(attempts,2);
  const failedBridge=backend(),failed=new NativeEngine({backend:failedBridge,pollMs:0,heartbeatMs:0});t.after(()=>failed.close());failedBridge.functions.subscribeMeter=()=>false;
  await assert.rejects(failed.initialize(),/meter.*unavailable|meter.*rejected/i);
});

test('native editor resize uses only the requested size mode and reports failed admission',async t=>{
  const {engine,bridge}=await setup(t);bridge.functions.resizeSamplerEditor=()=>({status:'accepted',width:1600,height:1000});
  await engine.resizeEditor('exp');assert.deepEqual(bridge.calls.at(-1),['resizeSamplerEditor','exp']);
  bridge.functions.resizeSamplerEditor=()=>null;await assert.rejects(engine.resizeEditor('min'),/resize.*admitted/i);
});

test('migrated key mapping uses native identities and native undo without a local closure entry',async t=>{
  const {engine,bridge}=await setup(t),store=new Store(engine);t.after(()=>store.close());
  const accepted=[];
  bridge.functions.setInstrumentZoneKeyRange=(instance,attachment,request,revision,map,zone,low,high)=>{
    accepted.push({kind:'map',instance,attachment,request,revision,map,zone,low,high});
    return {status:'accepted',job_id:request};
  };
  bridge.functions.undoInstrumentSession=(instance,attachment,request,revision)=>{
    accepted.push({kind:'undo',instance,attachment,request,revision});
    return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const command=accepted.find(item=>item.request===request);
    if(command.kind==='map'){
      const edited=yaml.replace('key_range: [36, 36]','key_range: [60, 60]');
      bridge.setSession(session(edited,8,connection({revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}})));
      return {state:'completed',request_id:request,revision:'1',undoCount:1,redoCount:0,error:''};
    }
    bridge.setSession(session(yaml,9,connection({revision:'2',nextRequestId:'3',history:{undoCount:0,redoCount:1}})));
    return {state:'completed',request_id:request,revision:'2',undoCount:0,redoCount:1,error:''};
  };
  await store.operation('Change zone Low','updateRule','kit.kick',{noteLo:60,noteHi:60});
  assert.deepEqual(accepted[0],{kind:'map',instance:'18446744073709551615',attachment:'9007199254740993',request:'1',revision:'0',map:'kit',zone:'kick',low:60,high:60});
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,60);
  assert.equal(store.undoStack.length,0);
  assert.equal(store.undoCount,1);
  assert.equal(bridge.calls.filter(call=>call[0]==='prepareSampler').length,0);
  await store.undo();
  assert.deepEqual(accepted[1],{kind:'undo',instance:'18446744073709551615',attachment:'9007199254740993',request:'2',revision:'1'});
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,36);
  assert.equal(store.undoCount,0);
});

test('whole-document edits preserve native zone identity for later mapping and undo',async t=>{
  const {engine,bridge}=await setup(t);await engine.renameNode('root','Renamed kit');
  const renamedYaml=engine.session.yaml,replacement=engine.connection,accepted=[];
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request,_revision,mapId,zoneId,low,high)=>{
    const map=parse(engine.session.yaml).assets.sample_maps.find(item=>item.id===mapId);
    if(!map?.zones.some(zone=>zone.id===zoneId))return `Unknown native zone: ${zoneId}`;
    accepted.push(['map',request,mapId,zoneId,low,high]);return {status:'accepted',job_id:request};
  };
  bridge.functions.undoInstrumentSession=(_instance,_attachment,request)=>{
    accepted.push(['undo',request]);return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const undo=accepted.at(-1)[0]==='undo';
    if(undo)bridge.setSession(session(renamedYaml,10,connection({instanceId:replacement.instanceId,attachmentId:replacement.attachmentId,revision:'2',nextRequestId:'3',history:{undoCount:0,redoCount:1}})));
    else{
      const graph=parse(renamedYaml),zone=graph.assets.sample_maps[0].zones.find(item=>item.id==='kick');
      zone.key_range=[60,60];
      bridge.setSession(session(stringify(graph),9,connection({instanceId:replacement.instanceId,attachmentId:replacement.attachmentId,revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}})));
    }
    const state=engine.backend.functions.getSamplerSession().connection;
    return {state:'completed',request_id:request,revision:state.revision,undoCount:state.history.undoCount,redoCount:state.history.redoCount,error:''};
  };
  await engine.updateRule('kit.kick',{noteLo:60,noteHi:60});
  assert.deepEqual(accepted[0],['map','1','kit','kick',60,60]);
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,60);
  await engine.undoSession();
  assert.deepEqual(accepted[1],['undo','2']);
  assert.equal(engine.getPatch().name,'Renamed kit');
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,36);
});

test('copied rules adopt their prepared native zone before migrated mapping and undo',async t=>{
  const {engine,bridge}=await setup(t),kick=engine.getPatch().rules.find(rule=>rule.id==='kit.kick');
  await engine.setRules([...engine.getPatch().rules,{...kick,id:'copied-kick',noteLo:60,noteHi:60}]);
  const copiedYaml=engine.session.yaml,replacement=engine.connection,accepted=[];
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request,_revision,mapId,zoneId,low,high)=>{
    accepted.push(['map',request,mapId,zoneId,low,high]);return {status:'accepted',job_id:request};
  };
  bridge.functions.undoInstrumentSession=(_instance,_attachment,request)=>{
    accepted.push(['undo',request]);return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const undo=accepted.at(-1)[0]==='undo',graph=parse(copiedYaml);
    if(!undo)graph.assets.sample_maps[0].zones.find(zone=>zone.id===accepted.at(-1)[3]).key_range=[61,61];
    const revision=undo?'2':'1',nextRequestId=undo?'3':'2',undoCount=undo?0:1;
    bridge.setSession(session(undo?copiedYaml:stringify(graph),undo?10:9,connection({instanceId:replacement.instanceId,attachmentId:replacement.attachmentId,revision,nextRequestId,history:{undoCount,redoCount:undo?1:0}})));
    return {state:'completed',request_id:request,revision,undoCount,redoCount:undo?1:0,error:''};
  };
  await engine.updateRule('copied-kick',{noteLo:61,noteHi:61});
  assert.deepEqual(accepted[0],['map','1','kit','copied-kick',61,61]);
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,36);
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='copied-kick').noteLo,61);
  await engine.undoSession();
  assert.deepEqual(accepted[1],['undo','2']);
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,36);
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='copied-kick').noteLo,60);
});

test('shipped round robin mapping edits every candidate through whole-document preparation',async t=>{
  const {engine,bridge}=await setup(t),store=new Store(engine);t.after(()=>store.close());
  const rule=engine.getPatch().rules.find(item=>item.source==='kit.hard_snare');
  assert.equal(engine.ownsOperation('updateRule',rule.id,{noteLo:39,noteHi:39}),false);
  await store.operation('Change round robin range','updateRule',rule.id,{noteLo:39,noteHi:39});
  const prepared=parse(bridge.calls.filter(call=>call[0]==='prepareSampler').at(-1)[1]);
  const candidates=prepared.assets.sample_maps[0].zones.filter(zone=>zone.round_robin_group==='kit.hard_snare');
  assert.equal(candidates.length,2);assert.deepEqual(candidates.map(zone=>zone.key_range),[[39,39],[39,39]]);
  assert.equal(engine.getPatch().rules.find(item=>item.source==='kit.hard_snare').noteLo,39);
  assert.equal(store.undoCount,1);
  const reopened=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});await reopened.initialize();t.after(()=>reopened.close());
  assert.equal(reopened.getPatch().rules.find(item=>item.source==='kit.hard_snare').noteLo,39);
  assert.equal(reopened.getPatch().selectors['kit.hard_snare'].candidates.length,2);
});

test('a successful native mapping retires an abandoned local redo branch',async t=>{
  const {engine,bridge}=await setup(t),store=new Store(engine);t.after(()=>store.close());
  await store.changeParam('drums.level',.4);await store.undo();assert.equal(store.canRedo,true);
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request)=>({status:'accepted',job_id:request});
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const edited=yaml.replace('key_range: [36, 36]','key_range: [60, 60]');
    bridge.setSession(session(edited,8,connection({revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}})));
    return {state:'completed',request_id:request,revision:'1',undoCount:1,redoCount:0,error:''};
  };
  await store.operation('Change zone Low','updateRule','kit.kick',{noteLo:60,noteHi:60});
  assert.equal(store.canRedo,false);
});

test('native undo preserves the intervening local command as the next mixed-history undo',async t=>{
  const {engine,bridge}=await setup(t),store=new Store(engine);t.after(()=>store.close());
  const accepted=[],originalLevel=engine.getPatch().params['drums.level'].value;let nativeEdits=0;
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request,_revision,_map,_zone,low)=>{
    accepted.push(['map',request,low]);return {status:'accepted',job_id:request};
  };
  bridge.functions.undoInstrumentSession=(_instance,_attachment,request)=>{
    accepted.push(['undo',request]);return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const undo=accepted.at(-1)[0]==='undo';
    if(undo)nativeEdits--;
    else nativeEdits++;
    const note=nativeEdits===0?36:nativeEdits===1?60:61;
    const text=yaml.replace('key_range: [36, 36]',`key_range: [${note}, ${note}]`);
    const revision=String(accepted.length),nextRequestId=String(accepted.length+1);
    bridge.setSession(session(text,7+accepted.length,connection({revision,nextRequestId,history:{undoCount:nativeEdits,redoCount:undo?1:0}})));
    return {state:'completed',request_id:request,revision,undoCount:nativeEdits,redoCount:undo?1:0,error:''};
  };
  await store.operation('Map first key','updateRule','kit.kick',{noteLo:60,noteHi:60});
  await store.changeParam('drums.level',.4);
  await store.operation('Map second key','updateRule','kit.kick',{noteLo:61,noteHi:61});
  assert.equal(store.undoLabel,'Change Key Range');
  await store.undo();
  assert.equal(store.undoLabel,'Change drums.level');
  await store.undo();
  assert.equal(engine.getPatch().params['drums.level'].value,originalLevel);
  assert.equal(accepted.filter(item=>item[0]==='undo').length,1);
});

test('a native mapping remains newest when bounded native history evicts its oldest entry',async t=>{
  const initial=session(yaml,7,connection({revision:'100',nextRequestId:'101',history:{undoCount:100,redoCount:0}}));
  const bridge=backend(initial),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});await engine.initialize();t.after(()=>engine.close());
  const store=new Store(engine);t.after(()=>store.close());const accepted=[];
  await store.changeParam('drums.level',.4);
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request)=>{accepted.push(['map',request]);return {status:'accepted',job_id:request};};
  bridge.functions.undoInstrumentSession=(_instance,_attachment,request)=>{accepted.push(['undo',request]);return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const undo=accepted.at(-1)[0]==='undo',text=undo?yaml:yaml.replace('key_range: [36, 36]','key_range: [60, 60]');
    const state=undo
      ? connection({revision:'102',nextRequestId:'103',history:{undoCount:99,redoCount:1}})
      : connection({revision:'101',nextRequestId:'102',history:{undoCount:100,redoCount:0}});
    const acceptedSession=session(text,undo?9:8,state);
    acceptedSession.document.parameters.find(parameter=>parameter.id==='drums.level').normalisedValue=.4;
    bridge.setSession(acceptedSession);
    return {state:'completed',request_id:request,revision:state.revision,undoCount:state.history.undoCount,redoCount:state.history.redoCount,error:''};
  };
  await store.operation('Change zone Low','updateRule','kit.kick',{noteLo:60,noteHi:60});
  assert.equal(store.undoLabel,'Change Key Range');await store.undo();
  assert.deepEqual(accepted.at(-1),['undo','102']);assert.equal(engine.getPatch().params['drums.level'].value,.4);
});

test('a failed native session command refreshes the consumed request identity before retry',async t=>{
  const {engine,bridge}=await setup(t),requests=[];let attempt=0;
  bridge.functions.setInstrumentZoneKeyRange=(instance,attachment,request)=>{
    requests.push(request);return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    attempt++;
    if(attempt===1){
      bridge.setSession(session(yaml,7,connection({nextRequestId:'2'})));
      return {state:'failed',request_id:request,revision:'0',undoCount:0,redoCount:0,error:'Preparation failed'};
    }
    const edited=yaml.replace('key_range: [36, 36]','key_range: [61, 61]');
    bridge.setSession(session(edited,8,connection({revision:'1',nextRequestId:'3',history:{undoCount:1,redoCount:0}})));
    return {state:'completed',request_id:request,revision:'1',undoCount:1,redoCount:0,error:''};
  };
  await assert.rejects(engine.updateRule('kit.kick',{noteLo:60,noteHi:60}),/Preparation failed/);
  await engine.updateRule('kit.kick',{noteLo:61,noteHi:61});
  assert.deepEqual(requests,['1','2']);
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,61);
});


test('comparison unmapped rules lose obsolete native ownership before later range edits',async t=>{
  const {engine,bridge}=await setup(t);
  await engine.updateRule('kit.kick',{source:null});
  const rule=engine.getPatch().rules.find(rule=>rule.id==='kit.kick');
  assert.equal(rule.source,null);assert.equal(rule.nativeMapId,undefined);assert.equal(rule.nativeZoneId,undefined);
  bridge.functions.setInstrumentZoneKeyRange=()=>assert.fail('an unbound rule cannot address a retired native zone');
  await engine.updateRule('kit.kick',{noteLo:70,noteHi:70});
  assert.equal(engine.getPatch().rules.find(rule=>rule.id==='kit.kick').noteLo,70);
  assert.equal(bridge.calls.filter(([method])=>method==='prepareSampler').length,2);
});


test('comparison sound duplication generates fresh zone identities without changing original candidate origins',async t=>{
  const {engine,bridge}=await setup(t),before=engine.getPatch();
  const node=before.nodes.find(node=>before.rules.some(rule=>rule.target===node.id&&rule.source==='kit.open_hats'));
  assert.ok(node,'packaged open-hat sound has a shared selector');
  const copied=await engine.duplicateNode(node.id);
  const graph=parse(bridge.calls.filter(([method])=>method==='prepareSampler').at(-1)[1]);
  const zones=graph.assets.sample_maps[0].zones,ids=zones.map(zone=>zone.id);
  assert.equal(new Set(ids).size,ids.length,'copied rules cannot reuse original candidate native zone IDs');
  for(const id of ['hat_open','hat_open_alt'])assert.ok(ids.includes(id),'original candidate identity remains');
  const rule=engine.getPatch().rules.find(rule=>rule.target===copied);
  assert.ok(ids.includes(rule.id),'the copied rule uses its fresh generated identity');
  assert.ok(ids.includes(rule.id+'_1'));
  const original=before.rules.find(rule=>rule.target===node.id);
  await engine.setRules([...before.rules,{...original,id:'undefined.undefined',nativeMapId:undefined,nativeZoneId:undefined}]);
  const authored=parse(bridge.calls.filter(([method])=>method==='prepareSampler').at(-1)[1]).assets.sample_maps[0].zones.map(zone=>zone.id);
  assert.equal(new Set(authored).size,authored.length,'a rule without native provenance cannot borrow candidate origins');
  assert.ok(authored.includes('undefined.undefined'));assert.ok(authored.includes('undefined.undefined_1'));
});
