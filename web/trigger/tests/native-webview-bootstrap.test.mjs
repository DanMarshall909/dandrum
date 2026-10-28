import test from 'node:test';
import assert from 'node:assert/strict';
import {installNativeWebViewTransport} from '../engine/native-webview-bootstrap.mjs';
import {selectNativeTransport} from '../engine/native-transport.mjs';
import {NativeEngine} from '../engine/native-engine.mjs';
import {session,editorProfile} from './native-fixture.mjs';

function host(config={}){
  const calls=[];
  const scope={__DANDRUM_NATIVE__:{protocolVersion:1,instanceId:'18446744073709551615',attachmentId:'9007199254740993',...config},ipc:{postMessage(message){
    assert.equal(this,scope.ipc);assert.ok(calls.length<8,'bounded native requests');calls.push(JSON.parse(message));
  }}};
  const deliver=body=>scope.__DANDRUM__.receiveNativeMessage(JSON.stringify({version:1,instanceId:'18446744073709551615',attachmentId:'9007199254740993',...body}));
  return {scope,calls,deliver};
}

test('absence preserves silent preview, declared native and comparison consumers',()=>{
  assert.equal(installNativeWebViewTransport(null),null);
  const preview={};assert.equal(installNativeWebViewTransport(preview),null);assert.equal(selectNativeTransport(preview),null);
  const direct={__DANDRUM__:{protocolVersion:1,backendId:'existing',backend:{getNativeFunction(){},addEventListener(){},removeEventListener(){}}}};
  assert.equal(installNativeWebViewTransport(direct),null);assert.equal(selectNativeTransport(direct).backendId,'existing');
  const comparison={__JUCE__:{backend:{getNativeFunction(){}}}};
  assert.equal(installNativeWebViewTransport(comparison),null);assert.equal(selectNativeTransport(comparison).backendId,'juce');
});

test('installation selects the bounded broker, binds IPC and delivers native replies and events',async()=>{
  const {scope,calls,deliver}=host({nextRequestId:'41'});
  const lifetime=installNativeWebViewTransport(scope),selected=selectNativeTransport(scope);
  assert.equal(selected.backendId,'juce-free');
  assert.deepEqual(selected.capabilities,{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false});
  const call=selected.backend.getNativeFunction('setParameter')('drums.level',.37,7);
  assert.deepEqual(calls,[{version:1,attachmentId:'9007199254740993',requestId:'41',method:'setParameter',args:['drums.level',.37,7]}]);
  assert.equal(deliver({requestId:'41',status:'accepted',result:{generation:7,value:.37}}),'reply');
  assert.deepEqual(await call,{generation:7,value:.37});
  const feedback=[];selected.backend.addEventListener('parameterStateChanged',value=>feedback.push(value));
  assert.equal(deliver({event:'parameterStateChanged',sequence:'1',payload:{generation:7,parameters:[{id:'drums.level',value:.19}]}}),'event');
  assert.deepEqual(feedback,[{generation:7,parameters:[{id:'drums.level',value:.19}]}]);
  const rejected=selected.backend.getNativeFunction('unsupported')();
  deliver({requestId:'42',status:'rejected',error:{code:'capabilityUnavailable',method:'unsupported'}});
  await assert.rejects(rejected,{code:'capabilityUnavailable',method:'unsupported'});
  assert.equal(lifetime.dispose(),true);assert.equal(lifetime.dispose(),false);
  assert.equal(deliver({event:'parameterStateChanged',sequence:'2',payload:{generation:8}}),'ignored');assert.equal(feedback.length,1);
});

test('same installation preserves outstanding work and cursor, and closed attachment cannot restart',async()=>{
  const {scope,calls,deliver}=host();const handle=installNativeWebViewTransport(scope);
  const backend=selectNativeTransport(scope).backend;
  const pending=backend.getNativeFunction('first')();
  scope.__DANDRUM_NATIVE__={...scope.__DANDRUM_NATIVE__};
  assert.equal(installNativeWebViewTransport(scope),handle);
  deliver({requestId:'1',status:'accepted',result:17});assert.equal(await pending,17);
  const second=backend.getNativeFunction('second')();const closed=assert.rejects(second,{code:'closed'});
  handle.dispose();await closed;
  assert.deepEqual(calls.map(call=>call.requestId),['1','2']);
  assert.throws(()=>installNativeWebViewTransport(scope),{code:'closed'});
  await assert.rejects(backend.getNativeFunction('third')(),{code:'closed'});assert.equal(calls.length,2);
});

test('present invalid bootstrap, identities and IPC fail before publishing or calling native',()=>{
  for(const config of [undefined,null,[],true,{}, {protocolVersion:2}]){
    const {scope,calls}=host();scope.__DANDRUM_NATIVE__=config;
    assert.throws(()=>installNativeWebViewTransport(scope),{code:'invalidNativeBootstrap'});assert.equal('__DANDRUM__' in scope,false);assert.deepEqual(calls,[]);
  }
  for(const field of ['instanceId','attachmentId','nextRequestId'])for(const value of ['0','01','18446744073709551616',1,null]){
    const {scope,calls}=host({[field]:value});assert.throws(()=>installNativeWebViewTransport(scope),{code:'invalidConfiguration'});assert.equal('__DANDRUM__' in scope,false);assert.deepEqual(calls,[]);
  }
  for(const ipc of [undefined,null,{}, {postMessage:1}]){
    const {scope}=host();scope.ipc=ipc;assert.throws(()=>installNativeWebViewTransport(scope),{code:'missingNativeIpc'});assert.equal('__DANDRUM__' in scope,false);
  }
});

test('foreign descriptors and changed installation ownership fail without replacing live work',async()=>{
  for(const descriptor of [undefined,null,{protocolVersion:1}]){
    const {scope,calls}=host();scope.__DANDRUM__=descriptor;
    assert.throws(()=>installNativeWebViewTransport(scope),{code:'nativeBootstrapConflict'});assert.equal(scope.__DANDRUM__,descriptor);assert.deepEqual(calls,[]);
  }
  for(const change of [scope=>{delete scope.__DANDRUM_NATIVE__;},scope=>{scope.__DANDRUM__={};},scope=>{scope.__DANDRUM_NATIVE__.instanceId='2';},scope=>{scope.__DANDRUM_NATIVE__.attachmentId='3';},scope=>{scope.__DANDRUM_NATIVE__.nextRequestId='4';},scope=>{scope.ipc={postMessage:scope.ipc.postMessage};},scope=>{scope.ipc.postMessage=()=>{};}]){
    const {scope,calls}=host();const handle=installNativeWebViewTransport(scope),descriptor=scope.__DANDRUM__;
    const pending=descriptor.backend.getNativeFunction('pending')();const closed=assert.rejects(pending,{code:'closed'});
    try{change(scope);assert.throws(()=>installNativeWebViewTransport(scope),{code:'nativeBootstrapConflict'});assert.equal(calls.length,1);}
    finally{handle.dispose();await closed;}
  }
});

test('failed descriptor publication leaves no installation and can be retried',()=>{
  const {scope,calls}=host();let reject=true;
  const target=new Proxy(scope,{set(object,key,value){if(reject&&key==='__DANDRUM__')throw Error('publication blocked');return Reflect.set(object,key,value);}});
  assert.throws(()=>installNativeWebViewTransport(target),/publication blocked/);assert.deepEqual(calls,[]);assert.equal('__DANDRUM__' in scope,false);
  reject=false;const handle=installNativeWebViewTransport(target);assert.equal(selectNativeTransport(scope).backendId,'juce-free');handle.dispose();
});

test('production selector and NativeEngine consume the initialized session and release pending work',async()=>{
  const previous=globalThis.window;const {scope,calls,deliver}=host();globalThis.window=scope;
  const handle=installNativeWebViewTransport();const engine=new NativeEngine({heartbeatMs:0});
  try{
    const initialized=engine.initialize();
    assert.deepEqual(calls,[{version:1,attachmentId:'9007199254740993',requestId:'1',method:'negotiateEditorProtocol',args:[2]}]);
    deliver({requestId:'1',status:'accepted',result:editorProfile()});
    await new Promise(resolve=>setImmediate(resolve));
    assert.deepEqual(calls[1],{version:2,attachmentId:'9007199254740993',requestId:'2',method:'getSamplerSession',args:[]});
    deliver({version:2,requestId:'2',status:'accepted',result:session()});await initialized;
    assert.equal(engine.getPatch().name,'Advanced Drum Kit');assert.equal(engine.capabilities.metering,false);
    deliver({event:'parameterStateChanged',sequence:'1',payload:{generation:7,sequence:1,parameters:[{id:'drums.level',value:.23}]}});
    assert.equal(engine.getPatch().params['drums.level'].value,.23);
    const waiting=engine.invoke('getSamplerSession');const closed=assert.rejects(waiting,{code:'closed'});
    engine.close();assert.equal(handle.dispose(),true);await closed;
    assert.equal(deliver({version:2,requestId:'3',status:'accepted',result:session()}),'ignored');
    assert.equal(engine.closed,true);assert.equal(handle.dispose(),false);
  }finally{engine.close();handle.dispose();globalThis.window=previous;}
});
