import test from 'node:test';
import assert from 'node:assert/strict';
import {NativeEngine} from '../engine/native-engine.mjs';
import {backend} from './native-fixture.mjs';
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
