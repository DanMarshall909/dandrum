import test from 'node:test';
import assert from 'node:assert/strict';
import {parse,stringify} from 'yaml';
import {emptyPatch} from '../engine/presets.mjs';
import {NativeEngine} from '../engine/native-engine.mjs';
import {Store} from '../engine/store.mjs';
import {patchFromSession,compileNativePatch} from '../engine/native-document.mjs';
import {backend,connection,session,yaml} from './native-fixture.mjs';

const retain={retainPublicSurface:true};
const original=parse(yaml);
const layout=graph=>graph.ports.map(({maps_to,maps_from,default:authored,...port})=>port);
const grouped={kick:1,snare:2,closed_hat:3,open_hat:4};
function emptySession(){
  const graph=parse(yaml),modules=[{id:'empty_audio',type:'audio_mixer',static:{channels:2}}];
  graph.ports.forEach((port,index)=>{
    if(port.direction==='input'){const id=`empty_control_${index}`;modules.push({id,type:'control_mixer',static:{channels:1}});port.maps_to=`${id}.inputs`;}
    else port.maps_from='empty_audio.mix';
  });
  graph.modules=modules;graph.connections=[];delete graph.assets;graph.metadata.name='Untitled';
  const result=session(stringify(graph));
  for(const parameter of result.document.parameters){
    const group=Object.entries(grouped).find(([name])=>parameter.id.startsWith(`drums.${name}.`))?.[1];
    if(group){parameter.scope='sampleGroup';parameter.controlGroup=group;}
    if(parameter.id==='drums.level')parameter.normalisedValue=.5;
  }
  return result;
}
function selectedDraft(snapshot){
  const patch=patchFromSession(snapshot),uri='selected/'+'1'.repeat(64)+'.wav';
  patch.assets.push({id:'picked',name:'picked.wav',uri,state:'loaded',cached:true,duration:4/32000,sampleRate:32000,channels:2,bits:16,frameCount:4});
  patch.regions.push({id:'picked.whole',assetId:'picked',name:'Whole',kind:'sample',start:0,end:1,root:48,fades:{in:0,out:0},loop:null,playback:'once'});
  patch.rules.push({id:'picked',group:'root',source:'picked.whole',name:'Picked',noteLo:48,noteHi:48,velLo:1,velHi:127,root:48});
  return patch;
}

test('native Empty compilation retains every public alias and root interface without sample resources',()=>{
  const snapshot=session();
  for(const parameter of snapshot.document.parameters){const group=Object.entries(grouped).find(([name])=>parameter.id.startsWith(`drums.${name}.`))?.[1];if(group){parameter.scope='sampleGroup';parameter.controlGroup=group;}}
  const patch=emptyPatch();patch.macros=[];patch.modules=[{id:'sample',type:'Sample player',params:{mode:'once'},paramIds:{}}];patch.voicePolicy.limit=16;
  const graph=parse(compileNativePatch(patch,snapshot,retain));
  assert.deepEqual(graph.preset_surface,original.preset_surface);
  assert.deepEqual(layout(graph),layout(original));
  assert.equal(graph.assets,undefined,'an accepted Empty has no dormant kit or selected resource');
  assert.ok(graph.modules.some(module=>module.type==='audio_mixer'));
  assert.ok(graph.modules.every(module=>['audio_mixer','control_mixer'].includes(module.type)));
});

test('a fresh native Empty snapshot supplies the complete interface for selected import without an earlier kit snapshot',()=>{
  const snapshot=emptySession(),patch=selectedDraft(snapshot);
  assert.equal(snapshot.document.sources.length,0);assert.equal(snapshot.document.maps.length,0);
  const graph=parse(compileNativePatch(patch,snapshot,retain));
  assert.deepEqual(graph.preset_surface,original.preset_surface);assert.deepEqual(layout(graph),layout(original));
  assert.deepEqual(graph.ports.map(port=>port.maps_to??port.maps_from),original.ports.map(port=>port.maps_to??port.maps_from),'restore target bindings from the retained public product descriptions');
  assert.deepEqual(graph.modules.map(module=>module.type),['midi_input','sample_map_player']);
  assert.equal(graph.assets.sample_sources.length,1);assert.equal(graph.assets.sample_sources[0].path,patch.assets[0].uri);
  assert.deepEqual(graph.assets.sample_maps[0].zones[0].key_range,[48,48]);
  assert.equal(graph.modules.find(module=>module.type==='sample_map_player').static.max_voices,16);
});

test('native Empty reconstruction refuses an unknown performance-control kind rather than inventing a target',()=>{
  const snapshot=emptySession(),graph=parse(snapshot.yaml);graph.preset_surface.parameters[0].name='custom.control';snapshot.yaml=stringify(graph);
  snapshot.document.parameters[0].id='custom.control';snapshot.document.parameters[0].name='custom.control';
  assert.throws(()=>compileNativePatch(selectedDraft(snapshot),snapshot,retain));
});

test('an Empty voice-policy edit cannot report success while discarding the requested policy',()=>{
  const snapshot=emptySession(),patch=patchFromSession(snapshot);patch.voicePolicy.limit=9;
  assert.throws(()=>compileNativePatch(patch,snapshot,retain));
});


test('native New Empty and a fresh consumer selected import use one authoritative session and native history',async t=>{
  const previous=globalThis.window,initial=session();initial.document.parameters=structuredClone(emptySession().document.parameters);
  const bridge=backend(initial),capabilities={metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false};
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  let engine=new NativeEngine({heartbeatMs:0,pollMs:0}),store,current=initial,candidate;
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  const expected=structuredClone(engine.getPatch().params);
  bridge.functions.prepareInstrumentSessionPatch=(_instance,attachment,request,revision,text)=>{
    assert.equal(attachment,current.connection.attachmentId);assert.equal(request,current.connection.nextRequestId);assert.equal(revision,current.connection.revision);
    const graph=parse(text);assert.deepEqual(graph.preset_surface,original.preset_surface);assert.deepEqual(layout(graph),layout(original));candidate=text;
    return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const revision=Number(current.connection.revision)+1;
    current=session(candidate,current.document.generation+1,connection({...current.connection,revision:String(revision),nextRequestId:String(BigInt(request)+1n),history:{undoCount:revision,redoCount:0}}));
    current.document.parameters=structuredClone(initial.document.parameters);
    for(const source of current.document.sources){Object.assign(source,{frameCount:'4',sampleRateHz:32000,channelCount:2});}
    bridge.setSession(current);return {state:'completed',request_id:request,revision:String(revision)};
  };
  await store.replacePatch('new');
  assert.deepEqual(engine.getPatch().params,expected);assert.equal(engine.getPatch().assets.length,0);assert.equal(engine.getPatch().rules.length,0);assert.equal(store.undoCount,1);
  store.close();engine.close();current.connection={...current.connection,attachmentId:'9007199254740994'};bridge.setSession(current);
  engine=new NativeEngine({heartbeatMs:0,pollMs:0});await engine.initialize();store=new Store(engine);
  assert.equal(parse(engine.session.yaml).modules.some(module=>module.type==='sample_map_player'),false);
  const hash='1'.repeat(64),resource=`selected/${hash}.wav`;
  bridge.functions.chooseSamplerFiles=()=>({selectionId:'1',phase:'choosing'});
  bridge.functions.getSamplerSelectionStatus=()=>({selectionId:'1',phase:'ready',files:[{name:'picked.wav',resourceId:resource,contentSha256:hash,encodedBytes:60,sampleRateHz:32000,channelCount:2,frameCount:4}]});
  bridge.functions.releaseSamplerSelection=()=>({selectionId:'1',phase:'released'});
  const result=await store.operation('Import samples','importSamples',await engine.chooseSamples(),{policy:'sequential',startNote:48,groupName:engine.getPatch().name});
  assert.deepEqual(engine.getPatch().params,expected);assert.equal(store.undoCount,2);assert.equal(result.assets.length,1);assert.equal(result.assets[0].uri,resource);
  assert.ok(engine.getPatch().rules.some(rule=>rule.id===result.rule));
  assert.deepEqual(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').map(call=>call[3]),['1','2']);
});


for(const method of ['removeRule','removeCandidate'])test(`native ${method} cannot silently turn retained assets into New Empty`,async t=>{
  const graph=parse(yaml);graph.modules.find(module=>module.type==='sample_map_player').static.max_voices=16;
  graph.assets.sample_maps[0].zones=[graph.assets.sample_maps[0].zones.find(zone=>zone.id==='snare_hard_a')];
  const initial=session(stringify(graph));initial.document.parameters=structuredClone(emptySession().document.parameters);
  const previous=globalThis.window,bridge=backend(initial),capabilities={metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false};
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});await engine.initialize();
  const before=engine.getPatch(),selector=Object.keys(before.selectors)[0];let admitted=0;
  bridge.functions.prepareInstrumentSessionPatch=()=>{admitted++;return Promise.reject({code:'unexpectedTestAdmission'});};
  const args=method==='removeRule'?[before.rules[0].id]:[selector,before.selectors[selector].candidates[0].id];
  await assert.rejects(engine[method](...args));
  assert.equal(admitted,0,'unrepresentable unmapping must reject before native admission, preserving every library asset');
  assert.deepEqual(engine.getPatch(),before);assert.equal(engine.sessionHistory.undoCount,0);
});


test('native removal of one mapping retains all sample assets and the remaining mappings',async t=>{
  const initial=session();initial.document.parameters=structuredClone(emptySession().document.parameters);
  const previous=globalThis.window,bridge=backend(initial),capabilities={metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false};
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});t.after(()=>{engine.close();globalThis.window=previous;});await engine.initialize();
  const before=engine.getPatch();let candidate;
  bridge.functions.prepareInstrumentSessionPatch=(_instance,_attachment,request,_revision,text)=>{candidate=text;return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const after=session(candidate,8,connection({revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}}));
    after.document.parameters=structuredClone(initial.document.parameters);bridge.setSession(after);return {state:'completed',request_id:request,revision:'1'};
  };
  await engine.removeRule('kit.kick');
  assert.deepEqual(engine.getPatch().assets,before.assets);assert.deepEqual(engine.getPatch().regions,before.regions);
  assert.equal(engine.getPatch().rules.some(rule=>rule.id==='kit.kick'),false);
  assert.equal(engine.getPatch().rules.length,before.rules.length-1);assert.equal(engine.sessionHistory.undoCount,1);
  assert.deepEqual(parse(candidate).assets.sample_maps[0].zones.map(zone=>zone.id),['snare_soft','snare_hard_a','snare_hard_b','hat_closed','hat_open','hat_open_alt']);
});
