import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';

const setup=()=>{const engine=new MockEngine({preset:'felt-kit',preparationMs:0,loadMs:0,analysisMs:0});const store=new Store(engine);return {engine,store,close:()=>{store.close();engine.close();}};};

test('subtree deletion and structural undo restore sounds, mapping and order without restoring unrelated live controls',async()=>{
  const {engine,store,close}=setup();const original=engine.getPatch();
  await store.operation('Delete Keys','removeNode','keys');
  assert.equal(engine.getPatch().rules.length,13);assert.equal(engine.getPatch().nodes.some(n=>n.parent==='keys'),false);
  engine.setParam('Cutoff',.77);await store.undo();
  assert.deepEqual(engine.getPatch().nodes,original.nodes);assert.deepEqual(engine.getPatch().rules,original.rules);
  assert.equal(engine.getPatch().params.Cutoff.value,.77);
  await store.redo();assert.equal(engine.getPatch().rules.length,13);
  const group=await store.operation('Add group','addNode','root','group');
  await store.operation('Rename group','renameNode',group,'Percussion');
  const sound=await engine.addNode(group,'sound');await engine.moveNode(sound,'drums',0);
  assert.equal(engine.getPatch().nodes.find(n=>n.id===sound).parent,'drums');
  const copy=await engine.duplicateNode('snare');assert.equal(engine.getPatch().rules.filter(r=>r.target===copy).length,2);
  await assert.rejects(engine.moveNode('root',group,0),/Invalid parent/);
  close();
});

test('regions and slice edits keep valid ranges and undo restores removed mapping',async()=>{
  const {engine,store,close}=setup();await engine.setRegion('k60f',{start:.1,end:.8,loop:{start:.3,end:.7,crossfade:.02}});
  assert.equal(engine.getPatch().regions.find(r=>r.id==='k60f').start,.1);
  await assert.rejects(engine.setRegion('k60f',{start:.9}),/range/);
  assert.equal(engine.getPatch().regions.find(r=>r.id==='k60f').start,.1);
  const newId=await store.operation('Split slice','splitSlice','slice-0',.05);
  assert.equal(engine.getPatch().regions.filter(r=>r.kind==='slice').length,9);
  await engine.mergeSlices('slice-0',newId);assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-0').end,.125);
  await store.operation('Delete slice','removeSlice','slice-2');assert.equal(engine.getPatch().rules.some(r=>r.source==='slice-2'),false);
  await store.undo();assert.equal(engine.getPatch().rules.find(r=>r.source==='slice-2').noteLo,26);
  await engine.updateRule('k60f',{noteLo:61,noteHi:67,root:61});assert.equal(engine.getPatch().rules.find(r=>r.id==='k60f').root,61);
  await assert.rejects(engine.updateRule('k60f',{noteLo:68}),/range/);
  await engine.setHistory('norm',{on:true});await engine.reorderHistory(['norm','fade','trim',...engine.getPatch().history.filter(op=>op.regionId==='k60f'&&!['norm','fade','trim','src'].includes(op.id)).map(op=>op.id),'src']);
  assert.equal(engine.getPatch().history[0].id,'norm');await assert.rejects(engine.removeHistory('src'),/Source/);
  close();
});

test('selectors, modulation, macro bindings and processors accept edits and enforce fixed-route safety',async()=>{
  const {engine,store,close}=setup();await engine.setSelector('snare-hard',{policy:'weighted'});
  await engine.setCandidate('snare-hard','hard-a',{muted:true,weight:3});await engine.reorderCandidates('snare-hard',['hard-c','hard-b','hard-a']);
  assert.equal(engine.getPatch().selectors['snare-hard'].candidates[2].weight,3);
  await assert.rejects(engine.removeRoute('route-0'),/fixed/);
  const route=await store.operation('Added route','addRoute',{source:'LFO 1',destination:'Filter resonance',amount:-.25,polarity:'Bi',curve:'Linear'});
  assert.equal(engine.getPatch().routes.find(r=>r.id===route).amount,-.25);
  await engine.updateRoute(route,{on:false,amount:.4});await engine.removeRoute(route);
  const mod=await engine.addModulator('lfo');await engine.setModulator(mod,{shape:'triangle',config:{rate:4}});await engine.removeModulator(mod);
  engine.setMacro('Tone',.83);await engine.bindMacro('Tone','Pan',[1,0]);await engine.renameMacro('Tone','Colour');
  await engine.learnMidi('Tone');await engine.receiveMidi(74,2);
  assert.deepEqual(engine.getPatch().macros[0].midi,{cc:74,channel:2});
  await engine.setOutput('snare','drums');await engine.setSend('snare','rev',.7);
  const processor=await engine.addProcessor('keys','Delay');await engine.moveProcessor(processor,'drums',0);await engine.bypass(processor,true);await engine.setProcessorParam(processor,'Time',417);
  assert.equal(engine.getPatch().chains[0].modules[0].params.Time,417);assert.equal(engine.getPatch().chains[0].modules[0].bypassed,true);
  await engine.setVoicePolicy({mode:'mono',limit:1});await engine.setModuleParam('filter','cutoff',.2);
  assert.equal(engine.getPatch().voicePolicy.limit,1);close();
});

test('asset loading, analysis failure/cancellation and silent voices report their real mock state',async()=>{
  const {engine,close}=setup();const assets=[];engine.on('asset',a=>assets.push([a.state,a.progress]));
  const loaded=await engine.importAssets([{name:'new_C4_v96_rr2.wav'}]);
  assert.equal(loaded[0].state,'loaded');assert.deepEqual(assets.slice(0,4),[['loading',0],['loading',.25],['loading',.62],['loading',1]]);
  assert.equal(engine.getPeaks(loaded[0].id,64).length,64);
  const unsupported=await engine.importAssets([{name:'take.m4a'}]);assert.equal(unsupported[0].state,'unsupported');
  const before=engine.getPatch().regions;engine.setMockSwitch('failNextAnalysis',true);
  const failed=new Promise(resolve=>engine.on('job',j=>{if(j.state==='failed')resolve(j);}));engine.analyse('amen','transients');
  assert.equal((await failed).result,null);assert.deepEqual(engine.getPatch().regions,before);
  const job=engine.analyse('amen','transients');engine.cancel(job.id);assert.equal(engine.getJobs().find(j=>j.id===job.id).state,'cancelled');
  let telemetry;engine.on('telemetry',t=>telemetry=t);engine.noteOn(38,120);
  assert.equal(telemetry.voices.active,1);assert.equal(telemetry.selectorPos['snare-hard'].last,'hard-a');
  engine.noteOff(38);engine.telemetry.tick(1);assert.equal(telemetry.voices.active,0);
  engine.audition('k60f',{start:.2,end:.7});engine.telemetry.tick(.5);assert.ok(telemetry.playheads[0].pos>.2);
  engine.setMockSwitch('missing',true);assert.equal(engine.getPatch().assets.find(a=>a.id==='k60f').state,'missing');
  close();
});
