import test from 'node:test';
import assert from 'node:assert/strict';
import {parse,stringify} from 'yaml';
import {NativeEngine} from '../engine/native-engine.mjs';
import {patchFromSession,compileNativePatch} from '../engine/native-document.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
import {selectNativeSamples} from '../shell/import-actions.mjs';
import {installComputerKeyboard} from '../shell/computer-keyboard.mjs';
import {JSDOM} from 'jsdom';
import {backend,session,connection,yaml} from './native-fixture.mjs';

const hash='2e484972f38876c961b3c23fb4952487dce3e18501c7fb0efd1b8cb7011734d5',resource=`selected/${hash}.wav`,selectionId='9007199254740993';
const selected=()=>({resourceId:resource,name:'distinct-selected.wav',contentSha256:hash,encodedBytes:60,sampleRateHz:32000,channelCount:2,frameCount:4});
const capabilities={metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false};
function prepared(text,generation,owner,value=.61){
  const result=session(text,generation,owner),graph=parse(text);
  for(const source of result.document.sources){
    const declaration=graph.assets.sample_sources.find(s=>s.id===source.id);
    if(declaration.path===resource)Object.assign(source,{sampleRateHz:32000,channelCount:2,frameCount:'4'});
  }
  result.document.parameters.find(p=>p.id==='drums.level').normalisedValue=value;
  return result;
}
function selectedDocument(){
  const graph=parse(yaml);graph.assets.sample_sources.push({id:'picked',path:resource,regions:[{id:'whole',start_frame:0,end_frame:4,root_note:48}]});
  graph.assets.sample_maps[0].zones.push({id:'picked',region:'picked.whole',key_range:[48,48],velocity_range:[1,127]});return stringify(graph);
}
async function setup(t,{text=yaml,autoChoose=true}={}){
  const previous=globalThis.window,start=session(text);start.document.parameters.find(p=>p.id==='drums.level').normalisedValue=.37;
  const bridge=backend(start);globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities,backend:bridge}};
  const engine=new NativeEngine({heartbeatMs:0,pollMs:0});let store;
  t.after(()=>{store?.close();engine.close();globalThis.window=previous;});await engine.initialize();store=new Store(engine);
  bridge.functions.chooseSamplerFiles=()=>({selectionId,phase:'choosing'});
  bridge.functions.getSamplerSelectionStatus=id=>{assert.equal(id,selectionId);return {selectionId:id,phase:'ready',files:[selected()]};};
  bridge.functions.releaseSamplerSelection=id=>({selectionId:id,phase:'released'});
  const files=autoChoose?await engine.chooseSamples():[];
  return {engine,bridge,store,files};
}

test('native selected source projection preserves a logical resource reference instead of inventing an absolute path',()=>{
  const snapshot=prepared(selectedDocument(),7,connection()),patch=patchFromSession(snapshot),asset=patch.assets.find(a=>a.id==='picked');
  assert.equal(asset.uri,resource);assert.deepEqual([asset.frameCount,asset.sampleRate,asset.channels,asset.duration],[4,32000,2,4/32000]);
});
test('native patch compilation retains admitted logical selected references',()=>{
  const snapshot=prepared(selectedDocument(),7,connection()),patch=patchFromSession(snapshot);patch.assets.find(a=>a.id==='picked').uri=resource;
  const graph=parse(compileNativePatch(patch,snapshot));assert.equal(graph.assets.sample_sources.find(s=>s.id==='picked').path,resource);
});

test('normal single-file native Import is one nonoverlapping native edit with canonical results and retained current values',async t=>{
  const {engine,bridge,store}=await setup(t,{autoChoose:false}),presenter=new ShellPresenter(store),ids=Object.keys(engine.getPatch().params);let document,action='edit';
  bridge.functions.prepareInstrumentSessionPatch=(instance,attachment,request,revision,text)=>{
    assert.equal(instance,engine.connection.instanceId);assert.equal(attachment,engine.connection.attachmentId);assert.equal(request,'1');assert.equal(revision,'0');document=text;
    const graph=parse(text),source=graph.assets.sample_sources.find(source=>source.path===resource);assert.ok(source);
    const zone=graph.assets.sample_maps[0].zones.find(zone=>zone.region.startsWith(source.id+'.'));
    assert.deepEqual(zone.key_range,[48,48],'the ordinary one-file flow cannot stack across the packaged kit');return {status:'accepted',job_id:request};
  };
  bridge.functions.undoInstrumentSession=(_i,_a,request)=>{action='undo';return {status:'accepted',job_id:request};};
  bridge.functions.redoInstrumentSession=(_i,_a,request)=>{action='redo';return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{
    const undo=action==='undo',revision=action==='edit'?1:undo?2:3;
    bridge.setSession(prepared(undo?yaml:document,7+revision,connection({revision:String(revision),nextRequestId:String(revision+1),history:{undoCount:undo?0:1,redoCount:undo?1:0}})));
    return {state:'completed',request_id:request,revision:String(revision)};
  };
  await selectNativeSamples(presenter);
  assert.equal(bridge.calls.filter(([method])=>method==='chooseSamplerFiles').length,1);
  assert.equal(store.getSnapshot().editor.lastImportError,null);
  const accepted=engine.getPatch(),asset=accepted.assets.find(asset=>asset.uri===resource);
  assert.ok(asset,'the canonical accepted patch contains the real selected resource');assert.deepEqual([asset.sampleRate,asset.channels,asset.frameCount],[32000,2,4]);
  assert.equal(accepted.params['drums.level'].value,.61);assert.deepEqual(Object.keys(accepted.params),ids);assert.equal(store.undoCount,1);
  assert.ok(accepted.rules.some(rule=>rule.id===store.getSnapshot().editor.zoneSel),'selection uses the canonical native rule identity');
  assert.deepEqual(bridge.calls.filter(([method])=>['prepareInstrumentSessionPatch','releaseSamplerSelection'].includes(method)).map(([method])=>method),['prepareInstrumentSessionPatch','releaseSamplerSelection']);
  assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection')[0][1],selectionId);
  await store.undo();assert.equal(engine.getPatch().assets.some(a=>a.uri===resource),false);assert.equal(engine.getPatch().params['drums.level'].value,.61);
  await store.redo();assert.deepEqual(engine.getPatch(),accepted);assert.equal(store.undoCount,1);
});

test('native import failure releases a valid selection lease while preserving accepted state and its error',async t=>{
  const {engine,bridge,files}=await setup(t),before=engine.getPatch(),failure={code:'sessionWorkerBusy'};
  bridge.functions.prepareInstrumentSessionPatch=()=>Promise.reject(failure);
  await assert.rejects(engine.importSamples(files,{policy:'sequential',startNote:48}),error=>error===failure);
  assert.deepEqual(engine.getPatch(),before);assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,1);
});
test('foreign-instance selected references cannot prepare or release another instance outcome',async t=>{
  const {engine,bridge,files}=await setup(t),before=engine.getPatch();
  await assert.rejects(engine.importSamples(files.map(file=>({...file,instanceId:'42'})),{policy:'sequential',startNote:48}),/Invalid native sample selection/);
  assert.deepEqual(engine.getPatch(),before);assert.equal(bridge.calls.filter(([method])=>['prepareInstrumentSessionPatch','releaseSamplerSelection'].includes(method)).length,0);
});


test('ordinary native single-file Import chooses the next unused note instead of colliding with an earlier mapping',async t=>{
  const {engine,bridge,store,files}=await setup(t,{text:yaml.replace('key_range: [36, 36]','key_range: [48, 48]')}),presenter=new ShellPresenter(store);let document;
  bridge.functions.prepareInstrumentSessionPatch=(_i,_a,request,_r,text)=>{
    document=text;const graph=parse(text),source=graph.assets.sample_sources.find(s=>s.path===resource);
    assert.deepEqual(graph.assets.sample_maps[0].zones.find(z=>z.region.startsWith(source.id+'.')).key_range,[49,49]);return {status:'accepted',job_id:request};
  };
  bridge.functions.getInstrumentSessionJobStatus=request=>{bridge.setSession(prepared(document,8,connection({revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}})));return {state:'completed',request_id:request,revision:'1'};};
  await presenter.queueImport(files);assert.equal(store.getSnapshot().editor.lastImportError,null);assert.ok(document);assert.equal(engine.getPatch().rules.some(r=>r.noteLo===49&&r.noteHi===49),true);
});
test('cancelling native multi-file mapping releases one outcome without adding history',async t=>{
  const {engine,bridge,store,files}=await setup(t),presenter=new ShellPresenter(store),before=engine.getPatch();
  presenter.queueImport([...files,...files]);assert.ok(store.getSnapshot().editor.importDialog);
  await presenter.buildModel().cancelImport();
  assert.equal(store.getSnapshot().editor.importDialog,null);assert.deepEqual(engine.getPatch(),before);assert.equal(store.undoCount,0);
  assert.deepEqual(bridge.calls.filter(([method])=>method==='releaseSamplerSelection'),[['releaseSamplerSelection',selectionId]]);
});
test('invalid selected-reference metadata releases its own known lease before any native patch admission',async t=>{
  const {engine,bridge,files}=await setup(t),before=engine.getPatch();
  await assert.rejects(engine.importSamples(files.map(file=>({...file,uri:'/tmp/browser-asserted.wav'})),{policy:'sequential',startNote:48}),/Invalid native sample selection/);
  assert.deepEqual(engine.getPatch(),before);assert.equal(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').length,0);
  assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,1);
});


test('native mapping dialog names the existing group without offering an unserialized group rename',async t=>{
  const {store,files}=await setup(t),presenter=new ShellPresenter(store);
  presenter.queueImport([...files,...files]);const model=presenter.buildModel();
  assert.equal(model.importGroupNameEditable,false);assert.equal(model.importOptions.group,'kit');assert.equal(model.importOptions.groupName,'kit');
});
test('a fully mapped native keyboard refuses an automatic import and releases its selection',async t=>{
  const graph=parse(yaml);
  for(const zone of graph.assets.sample_maps[0].zones)zone.key_range=zone.id==='kick'?[0,35]:zone.id.startsWith('snare')?[36,36]:zone.id==='hat_closed'?[37,126]:[127,127];
  const {bridge,store,files}=await setup(t,{text:stringify(graph)}),presenter=new ShellPresenter(store);
  await presenter.queueImport(files);
  assert.match(store.getSnapshot().editor.lastImportError,/No free MIDI note/);
  assert.equal(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').length,0);
  assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,1);
});


test('native chooser admission errors stay visible in the production presenter',async t=>{
  const {engine,bridge,store}=await setup(t,{autoChoose:false}),presenter=new ShellPresenter(store),before=engine.getPatch();
  bridge.functions.chooseSamplerFiles=()=>Promise.reject({code:'sessionWorkerBusy'});
  await selectNativeSamples(presenter);
  assert.equal(store.getSnapshot().editor.lastImportError,'sessionWorkerBusy');assert.deepEqual(engine.getPatch(),before);
});
for(const retirement of ['store','view'])test(`retired native import presentation receives no late chooser callback after ${retirement}`,async t=>{
  const {engine,bridge,store}=await setup(t,{autoChoose:false}),presenter=new ShellPresenter(store),entered=Promise.withResolvers(),completion=Promise.withResolvers(),updates=[];
  t.after(()=>completion.resolve({selectionId,phase:'cancelled'}));
  bridge.functions.getSamplerSelectionStatus=()=>{entered.resolve();return completion.promise;};
  const setState=presenter.setState.bind(presenter);presenter.setState=update=>{updates.push(update);setState(update);};
  const pending=selectNativeSamples(presenter);await Promise.race([entered.promise,pending.then(()=>assert.fail('selection completed before status'))]);
  store.close();if(retirement==='view')engine.close();completion.resolve({selectionId,phase:'ready',files:[selected()]});await pending;
  assert.deepEqual(updates,[]);assert.equal(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').length,0);
});


test('accepted native Import stays successful if only terminal selection cleanup fails',async t=>{
  const {engine,bridge,store}=await setup(t,{autoChoose:false}),presenter=new ShellPresenter(store);let document;
  bridge.functions.prepareInstrumentSessionPatch=(_i,_a,request,_r,text)=>{document=text;return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=request=>{bridge.setSession(prepared(document,8,connection({revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}})));return {state:'completed',request_id:request,revision:'1'};};
  bridge.functions.releaseSamplerSelection=()=>Promise.reject({code:'releaseUnavailable'});
  await selectNativeSamples(presenter);
  assert.ok(engine.getPatch().assets.some(asset=>asset.uri===resource));assert.equal(store.undoCount,1);
  assert.match(store.getSnapshot().editor.modNote??'',/Sample imported.*releaseUnavailable/);
  assert.equal(store.getSnapshot().editor.lastImportError,null,'a completed native edit must not be reported as a failed import');
  assert.equal(bridge.calls.filter(([method])=>method==='prepareInstrumentSessionPatch').length,1);
});
for(const retirement of ['store','view'])test(`admitted native Import has no late presentation after ${retirement} retirement`,async t=>{
  const {engine,bridge,store}=await setup(t,{autoChoose:false}),presenter=new ShellPresenter(store),entered=Promise.withResolvers(),completion=Promise.withResolvers(),updates=[];let document;
  t.after(()=>completion.resolve({state:'failed',error:'test ended'}));
  const original=presenter.setState.bind(presenter);presenter.setState=update=>{updates.push(update);original(update);};
  bridge.functions.prepareInstrumentSessionPatch=(_i,_a,request,_r,text)=>{document=text;return {status:'accepted',job_id:request};};
  bridge.functions.getInstrumentSessionJobStatus=()=>{entered.resolve();return completion.promise;};
  const pending=selectNativeSamples(presenter);await Promise.race([entered.promise,pending.then(()=>assert.fail('import finished before native completion'))]);
  store.close();if(retirement==='view')engine.close();updates.length=0;
  const snapshots=bridge.calls.filter(([method])=>method==='getSamplerSession').length;
  bridge.setSession(prepared(document,8,connection({revision:'1',nextRequestId:'2',history:{undoCount:1,redoCount:0}})));
  completion.resolve({state:'completed',request_id:'1',revision:'1'});await pending;
  assert.deepEqual(updates,[]);
  if(retirement==='view'){
    assert.equal(bridge.calls.filter(([method])=>method==='getSamplerSession').length,snapshots);
    assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,0);
  }
});


test('native preparation and cleanup failure preserves the primary rejection and the prior accepted patch',async t=>{
  const {engine,bridge,files}=await setup(t),before=engine.getPatch(),failure={code:'sessionWorkerBusy'};
  bridge.functions.prepareInstrumentSessionPatch=()=>Promise.reject(failure);
  bridge.functions.releaseSamplerSelection=()=>Promise.reject({code:'releaseUnavailable'});
  await assert.rejects(engine.importSamples(files,{policy:'sequential',startNote:48}),error=>error.cause===failure&&error.message.includes('sessionWorkerBusy')&&error.message.includes('releaseUnavailable'));
  assert.deepEqual(engine.getPatch(),before);assert.equal(engine.sessionHistory.undoCount,0);
});


function keyboard(presenter){
  const dom=new JSDOM('<body></body>');
  window.addEventListener=dom.window.addEventListener.bind(dom.window);
  window.removeEventListener=dom.window.removeEventListener.bind(dom.window);
  presenter.attach();const uninstall=installComputerKeyboard(presenter.keyboardBindings);
  return {press:(key,options={})=>dom.window.document.body.dispatchEvent(new dom.window.KeyboardEvent('keydown',{key,bubbles:true,...options})),
    close:()=>{uninstall();presenter.detach();dom.window.close();}};
}
test('Escape disposes the native mapping choice once through the installed keyboard entrypoint',async t=>{
  const {engine,bridge,store,files}=await setup(t),presenter=new ShellPresenter(store),before=engine.getPatch(),keys=keyboard(presenter);
  try{
    presenter.queueImport([...files,...files]);const old=presenter.buildModel();
    keys.press('Escape');keys.press('Escape');await new Promise(setImmediate);
    assert.equal(store.getSnapshot().editor.importDialog,null);
    assert.deepEqual(bridge.calls.filter(([method])=>method==='releaseSamplerSelection'),[['releaseSamplerSelection',selectionId]]);
    await old.cancelImport();assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,1);
    assert.deepEqual(engine.getPatch(),before);assert.equal(store.undoCount,0);
  }finally{keys.close();}
});
test('replacing an open native mapping dialog disposes its previous choice once and retains the new choice',async t=>{
  const {engine,bridge,store}=await setup(t,{autoChoose:false}),presenter=new ShellPresenter(store),before=engine.getPatch(),keys=keyboard(presenter);let next=0,choosing;
  bridge.functions.chooseSamplerFiles=()=>({selectionId:String(++next),phase:'choosing'});
  bridge.functions.getSamplerSelectionStatus=id=>({selectionId:id,phase:'ready',files:[selected(),{...selected(),name:'other.wav'}]});
  presenter.browseSamples=()=>{choosing=selectNativeSamples(presenter);};
  try{
    keys.press('o',{ctrlKey:true});await choosing;const old=presenter.buildModel();
    assert.equal(store.getSnapshot().editor.importDialog.files[0].selectionId,'1');
    keys.press('o',{ctrlKey:true});await choosing;
    assert.deepEqual(bridge.calls.filter(([method])=>method==='releaseSamplerSelection'),[['releaseSamplerSelection','1']]);
    assert.equal(store.getSnapshot().editor.importDialog.files[0].selectionId,'2');
    await old.cancelImport();assert.equal(store.getSnapshot().editor.importDialog.files[0].selectionId,'2','an obsolete dialog callback cannot cancel its replacement');
    await presenter.buildModel().cancelImport();await old.cancelImport();
    assert.deepEqual(bridge.calls.filter(([method])=>method==='releaseSamplerSelection'),[['releaseSamplerSelection','1'],['releaseSamplerSelection','2']]);
    assert.deepEqual(engine.getPatch(),before);assert.equal(store.undoCount,0);
  }finally{keys.close();}
});


test('retired native mapping presentation ignores repeated cancellation and late cleanup failure',async t=>{
  const {bridge,store,files}=await setup(t),presenter=new ShellPresenter(store),completion=Promise.withResolvers(),updates=[];
  t.after(()=>completion.reject({code:'releaseUnavailable'}));
  presenter.queueImport([...files,...files]);const model=presenter.buildModel();
  bridge.functions.releaseSamplerSelection=()=>completion.promise;
  const cancelled=model.cancelImport();store.close();
  const original=presenter.setState.bind(presenter);presenter.setState=update=>{updates.push(update);original(update);};
  const repeated=model.cancelImport();completion.reject({code:'releaseUnavailable'});await Promise.all([cancelled,repeated]);
  assert.deepEqual(updates,[]);assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,1);
});
test('native dialog replacement retains a cleanup warning without rejecting the new mapping choice',async t=>{
  const {engine,bridge,store,files}=await setup(t),presenter=new ShellPresenter(store),before=engine.getPatch();
  presenter.queueImport([...files,...files]);bridge.functions.releaseSamplerSelection=()=>Promise.reject({code:'releaseUnavailable'});
  const replacement=files.map(file=>({...file,selectionId:'2'}));await presenter.queueImport([...replacement,...replacement]);
  assert.equal(store.getSnapshot().editor.importDialog.files[0].selectionId,'2');
  assert.match(store.getSnapshot().editor.modNote??'',/releaseUnavailable/);
  assert.deepEqual(bridge.calls.filter(([method])=>method==='releaseSamplerSelection'),[['releaseSamplerSelection',selectionId]]);
  assert.deepEqual(engine.getPatch(),before);assert.equal(store.undoCount,0);
});


test('an empty import offer leaves the current native mapping choice live',async t=>{
  const {bridge,store,files}=await setup(t),presenter=new ShellPresenter(store);
  presenter.queueImport([...files,...files]);const dialog=store.getSnapshot().editor.importDialog;
  await presenter.queueImport([]);assert.equal(store.getSnapshot().editor.importDialog,dialog);
  assert.equal(bridge.calls.filter(([method])=>method==='releaseSamplerSelection').length,0);
});
test('native mapping replacement snapshots offered files before waiting for previous-choice disposal',async t=>{
  const {bridge,store,files}=await setup(t),presenter=new ShellPresenter(store),completion=Promise.withResolvers();
  t.after(()=>completion.resolve({selectionId,phase:'released'}));presenter.queueImport([...files,...files]);
  bridge.functions.releaseSamplerSelection=()=>completion.promise;
  const offered=[...files,...files].map(file=>({...file,selectionId:'2'}));
  const replacing=presenter.queueImport(offered);offered.length=0;completion.resolve({selectionId,phase:'released'});await replacing;
  assert.equal(store.getSnapshot().editor.importDialog?.files.length,2);
  assert.equal(store.getSnapshot().editor.importDialog.files[0].selectionId,'2');
  assert.deepEqual(bridge.calls.filter(([method])=>method==='releaseSamplerSelection'),[['releaseSamplerSelection',selectionId]]);
});
