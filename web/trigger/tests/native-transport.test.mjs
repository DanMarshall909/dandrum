import test from 'node:test';
import assert from 'node:assert/strict';
import {NativeEngine} from '../engine/native-engine.mjs';
import {backend,session,connection,yaml} from './native-fixture.mjs';
import {Store} from '../engine/store.mjs';
import {selectNativeTransport} from '../engine/native-transport.mjs';

test('the comparison host default backend preserves native calls, feedback and cleanup',async t=>{
  const previous=globalThis.window,bridge=backend();
  globalThis.window={__JUCE__:{backend:bridge}};
  t.after(()=>{globalThis.window=previous;});
  const engine=new NativeEngine({heartbeatMs:0});t.after(()=>engine.close());
  assert.equal(engine.backendId,'juce');
  await engine.initialize();await engine.setParam('drums.level',.23);
  assert.ok(bridge.calls.some(call=>JSON.stringify(call)===JSON.stringify(['setParameter','drums.level',.23,7])));
  bridge.events.get('parameterStateChanged')({generation:7,sequence:2,parameters:[{id:'drums.level',name:'Level',value:.61}]});
  assert.equal(engine.getPatch().params['drums.level'].value,.61);
  engine.close();assert.equal(bridge.events.size,0);
});

test('only absent native hosts select the silent browser preview',()=>{
  for(const scope of [undefined,{}, {__JUCE__:{}},{__JUCE__:{backend:{getNativeFunction:true}}}])assert.equal(selectNativeTransport(scope),null);
});

test('incompatible native protocol versions fail without falling back to JUCE or preview',()=>{
  const comparison=backend();
  for(const protocolVersion of [undefined,0,2,'1']){
    assert.throws(()=>selectNativeTransport({__DANDRUM__:{protocolVersion,backendId:'juce-free',backend:backend()},__JUCE__:{backend:comparison}}),/Unsupported native transport protocol version/);
  }
  assert.throws(()=>selectNativeTransport({__DANDRUM__:null}),/Unsupported native transport protocol version/);
  assert.deepEqual(comparison.calls,[]);
});

test('a declared native host must expose its identity and callable transport contract',()=>{
  for(const descriptor of [{protocolVersion:1},{protocolVersion:1,backendId:''},{protocolVersion:1,backendId:7},{protocolVersion:1,backendId:'juce-free',backend:{}},
    ...['getNativeFunction','addEventListener','removeEventListener'].map(method=>({protocolVersion:1,backendId:'juce-free',backend:{...backend(),[method]:null}}))]){
    assert.throws(()=>selectNativeTransport({__DANDRUM__:descriptor}),/Invalid native transport contract/);
  }
});

test('native invocation preserves comparison host rejection and absolute preset paths',async t=>{
  const bridge=backend(),engine=new NativeEngine({backend:bridge,heartbeatMs:0});t.after(()=>engine.close());
  bridge.functions.unavailable=()=> 'Native operation unavailable';
  bridge.functions.rejected=()=>({status:'busy'});
  await assert.rejects(engine.invoke('unavailable'),/Native operation unavailable/);
  await assert.rejects(engine.invoke('rejected'),/Native rejected rejected: busy/);
  assert.equal(await engine.invoke('getSamplerPresetPath'),'/repo/examples/patches/advanced-drum-kit.yaml');
  assert.throws(()=>new NativeEngine({backend:{}}),/backend unavailable/);
});

test('the versioned backend takes precedence over the comparison host and carries native capability errors',async t=>{
  const previous=globalThis.window,bridge=backend(),comparison=backend();
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',backend:bridge},__JUCE__:{backend:comparison}};
  t.after(()=>{globalThis.window=previous;});
  const engine=new NativeEngine({heartbeatMs:0});t.after(()=>engine.close());
  await engine.initialize();await engine.setParam('drums.level',.37);
  assert.equal(engine.backendId,'juce-free');assert.deepEqual(comparison.calls,[]);
  assert.ok(bridge.calls.some(call=>JSON.stringify(call)===JSON.stringify(['setParameter','drums.level',.37,7])));
  const before=engine.getPatch(),failure=Object.assign(new Error('Sample selection unavailable'),{code:'capabilityUnavailable',method:'chooseSamplerFiles'});
  bridge.functions.chooseSamplerFiles=()=>{throw failure;};
  await assert.rejects(engine.chooseSamples(),error=>error===failure&&error.code==='capabilityUnavailable'&&error.method==='chooseSamplerFiles');
  assert.deepEqual(engine.getPatch(),before);
});

test('a native initialization capability failure retains its structured identity',async t=>{
  const bridge=backend(),failure=Object.assign(new Error('Session snapshots unavailable'),{code:'capabilityUnavailable',method:'getSamplerSession'});
  bridge.functions.getSamplerSession=()=>Promise.reject(failure);
  const engine=new NativeEngine({backend:bridge,heartbeatMs:0});t.after(()=>engine.close());
  const before=engine.getPatch();
  await assert.rejects(engine.initialize(),error=>error===failure&&error.code==='capabilityUnavailable'&&error.method==='getSamplerSession');
  assert.deepEqual(engine.getPatch(),before);
});


test('an explicitly absent optional profile initializes the accepted native session without optional calls',async t=>{
  const previous=globalThis.window,bridge=backend();
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:bridge}};
  t.after(()=>{globalThis.window=previous;});
  const engine=new NativeEngine({heartbeatMs:0});t.after(()=>engine.close());
  await engine.initialize();
  assert.equal(engine.getPatch().name,'Advanced Drum Kit');
  assert.equal(engine.getPatch().params['drums.level'].value,.25);
  assert.deepEqual(bridge.calls.map(call=>call[0]),['getSamplerSession']);
  assert.deepEqual(engine.capabilities,{analysis:false,zeroCrossings:false,fx:false,modulation:false,wholeSourceAudition:false,metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false});
  assert.throws(()=>{engine.capabilities.metering=true;},TypeError);
  engine.close();assert.equal(bridge.events.size,0);
  assert.deepEqual(bridge.calls.map(call=>call[0]),['getSamplerSession']);
});


test('capability omission retains full services for every backend identity and invalid metadata fails before native calls',async t=>{
  const previous=globalThis.window;t.after(()=>{globalThis.window=previous;});
  for(const backendId of ['juce-free','comparison','future-host'])for(const capabilities of [undefined,{}, {metering:true}]){
    const bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId,backend:bridge,...(capabilities===undefined?{}:{capabilities})}};
    const engine=new NativeEngine({heartbeatMs:0});await engine.initialize();
    assert.deepEqual(bridge.calls.map(call=>call[0]),['getSamplerSession','setTriggerWindowTitle','subscribeMeter']);
    assert.equal(engine.capabilities.sourceWaveforms,true);assert.equal(engine.capabilities.editorResize,true);
    engine.close();
  }
  for(const name of ['metering','sourceWaveforms','windowTitle','editorResize']){
    const bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',backend:bridge,capabilities:{[name]:false}}};
    const engine=new NativeEngine({heartbeatMs:0});await engine.initialize();
    assert.deepEqual(bridge.calls.map(call=>call[0]),['getSamplerSession',...(name==='windowTitle'?[]:['setTriggerWindowTitle']),...(name==='metering'?[]:['subscribeMeter'])]);
    for(const field of ['metering','sourceWaveforms','windowTitle','editorResize'])assert.equal(engine.capabilities[field],field!==name);
    engine.close();
  }
  for(const capabilities of [undefined,null,[],false,'none',...['metering','sourceWaveforms','windowTitle','editorResize'].flatMap(name=>[null,0,'false',undefined].map(value=>({[name]:value})))]){
    const bridge=backend();globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',backend:bridge,capabilities}};
    assert.throws(()=>new NativeEngine(),/Invalid native transport capabilities/);assert.deepEqual(bridge.calls,[]);
  }
});

test('selected capability metadata is immutable and independent of later descriptor mutation',()=>{
  const capabilities={metering:false},transport=selectNativeTransport({__DANDRUM__:{protocolVersion:1,backendId:'juce-free',backend:backend(),capabilities}});
  capabilities.metering=true;assert.equal(transport.capabilities.metering,false);
  assert.throws(()=>{transport.capabilities.sourceWaveforms=false;},TypeError);
});


test('explicit absent waveform and resize requests preserve accepted state and report capability errors without native requests',async t=>{
  const previous=globalThis.window,bridge=backend();
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:bridge}};
  t.after(()=>{globalThis.window=previous;});
  const engine=new NativeEngine({heartbeatMs:0});t.after(()=>engine.close());await engine.initialize();
  const before=engine.getPatch(),history=engine.sessionHistory,events=[];engine.on('patch',event=>events.push(event));
  await assert.rejects(engine.getPeaksAsync('drums',64),error=>error.code==='capabilityUnavailable'&&error.method==='requestSourceWaveform');
  await assert.rejects(engine.resizeEditor('exp'),error=>error.code==='capabilityUnavailable'&&error.method==='resizeSamplerEditor');
  assert.deepEqual(Array.from(engine.getPeaks('drums',64)),[]);await engine.pollTelemetry();
  assert.deepEqual(engine.getPatch(),before);assert.deepEqual(engine.sessionHistory,history);assert.deepEqual(events,[]);
  assert.deepEqual(bridge.calls.map(call=>call[0]),['getSamplerSession']);
});


test('absent metering starts with unknown native voices and keeps mapping undo refresh and notes usable',async t=>{
  const previous=globalThis.window,bridge=backend();
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:bridge}};
  t.after(()=>{globalThis.window=previous;});
  const engine=new NativeEngine({heartbeatMs:5,pollMs:0});t.after(()=>engine.close());await engine.initialize();
  const store=new Store(engine);t.after(()=>store.close());
  assert.equal(store.getTelemetry().voices.active,null);assert.equal(store.getTelemetry().voices.max,8);
  assert.equal(store.getTelemetry().voiceCountAvailable,false);assert.deepEqual(store.getTelemetry().meters,{});
  let undo=false;
  bridge.functions.setInstrumentZoneKeyRange=(_instance,_attachment,request)=>({status:'accepted',job_id:request});
  bridge.functions.undoInstrumentSession=(_instance,_attachment,request)=>{undo=true;return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const revision=undo?'2':'1';
    bridge.setSession(session(undo?yaml:yaml.replace('key_range: [36, 36]','key_range: [60, 60]'),undo?9:8,connection({revision,nextRequestId:undo?'3':'2',history:{undoCount:undo?0:1,redoCount:undo?1:0}})));
    return {state:'completed',request_id:request,revision};
  };
  await store.operation('Change key range','updateRule','kit.kick',{noteLo:60,noteHi:60});
  assert.equal(engine.getPatch().rules[0].noteLo,60);assert.equal(store.undoCount,1);assert.equal(store.undoStack.length,0);
  await store.undo();assert.equal(engine.getPatch().rules[0].noteLo,36);assert.equal(store.undoCount,0);
  bridge.setSession(session(yaml,10,engine.session.connection));
  await bridge.events.get('parameterStateChanged')({generation:10,sequence:0,parameters:[]});
  await engine.setParam('drums.level',.6);await engine.noteOn(36,100);
  await new Promise(resolve=>setTimeout(resolve,65));
  assert.ok(bridge.calls.some(call=>call[0]==='noteHeartbeat'));
  engine.close();engine.close();await new Promise(resolve=>setTimeout(resolve,5));
  assert.deepEqual(bridge.calls.filter(call=>call[0]==='noteOff'),[['noteOff',36,10]]);
  assert.ok(bridge.calls.some(call=>JSON.stringify(call)===JSON.stringify(['setParameter','drums.level',.6,10])));
  assert.equal(bridge.events.size,0);const closedCalls=bridge.calls.length;await new Promise(resolve=>setTimeout(resolve,60));assert.equal(bridge.calls.length,closedCalls);
  assert.deepEqual(bridge.calls.filter(call=>['subscribeMeter','getMeterPacket','ackMeterPacket','setMeterVisible','setTriggerWindowTitle','requestSourceWaveform','resizeSamplerEditor'].includes(call[0])),[]);
});

test('accepted native voice-limit changes notify Store telemetry without a meter service',async t=>{
  const previous=globalThis.window;t.after(()=>{globalThis.window=previous;});
  for(const publication of ['generation','prepared']){
    const bridge=backend();
    globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:bridge}};
    const engine=new NativeEngine({heartbeatMs:0,pollMs:0});await engine.initialize();
    const store=new Store(engine),notified=[];t.after(()=>{store.close();engine.close();});
    assert.deepEqual(store.getTelemetry().voices,{active:null,max:8});
    const unsubscribe=store.subscribeTelemetry(()=>notified.push(store.getTelemetry()));t.after(unsubscribe);
    if(publication==='generation'){
      bridge.setSession(session(yaml.replace('max_voices: 8','max_voices: 12'),8,engine.session.connection));
      await bridge.events.get('parameterStateChanged')({generation:8,sequence:0,parameters:[]});
    }else await engine.setVoicePolicy({limit:12});
    assert.equal(store.getSnapshot().patch.voicePolicy.limit,12,publication);
    assert.deepEqual(store.getTelemetry().voices,{active:null,max:12},publication);
    assert.equal(store.getTelemetry().voiceCountAvailable,false);assert.deepEqual(store.getTelemetry().meters,{});
    assert.ok(notified.some(snapshot=>snapshot.voices.max===12&&snapshot.voices.active===null),publication);
    assert.deepEqual(engine.getTelemetrySnapshot(),store.getTelemetry());
    await engine.pollTelemetry();
    assert.deepEqual(bridge.calls.filter(call=>['subscribeMeter','getMeterPacket','ackMeterPacket','setMeterVisible','setTriggerWindowTitle','requestSourceWaveform','resizeSamplerEditor'].includes(call[0])),[]);
    store.close();engine.close();
  }
});
