import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
const setup=()=>{const engine=new MockEngine({preset:'felt-kit',preparationMs:0,loadMs:0,analysisMs:0}),store=new Store(engine),presenter=new ShellPresenter(store);return {engine,store,presenter,close:()=>{store.close();engine.close();}};};
test('Lush and a selected nested candidate preview their own source without triggering neighbouring rules',t=>{
 const {engine,presenter,close}=setup();t.after(close);presenter.buildModel().srcGroups.find(g=>g.id==='patch').items[0].play();
 assert.deepEqual(engine.telemetry.active.map(v=>v.layer),['module:lush']);engine.telemetry.reset();
 presenter.go('vel');presenter.setState({selectorTarget:'snare',candidateSelection:'snare-hard'});
 presenter.buildModel().insp.rows.find(r=>r.label==='Preview').action();assert.deepEqual(engine.telemetry.active.map(v=>v.layer),['hard-a']);
 engine.telemetry.reset();presenter.auditionSelection();assert.deepEqual(engine.telemetry.active.map(v=>v.layer),['hard-a']);
});
test('Voice module inspector and knob share Cutoff, including undo in either direction and added module sections',async()=>{
 const {engine,store,presenter,close}=setup();try{presenter.go('voice');await store.operation('Module Cutoff','setModuleParam','filter','cutoff',.9);
 const cutoff=()=>presenter.buildModel().voiceSecs.find(s=>s.title==='Filter').knobs.find(k=>k.label==='Cutoff');assert.equal(cutoff().v,.9);cutoff().set(.7);assert.equal(engine.getPatch().modules.find(m=>m.id==='filter').params.cutoff,.7);await store.undo();assert.equal(cutoff().v,.9);await store.undo();assert.equal(cutoff().v,.42);
 await presenter.buildModel().template.add('Filter');const added=engine.getPatch().modules.at(-1),section=presenter.buildModel().voiceSecs.find(s=>s.id.startsWith(added.id));assert.ok(section);section.knobs[0].set(.2);assert.equal(engine.getPatch().modules.at(-1).params.cutoff,.2);assert.equal(cutoff().v,.42);
 }finally{close();}
});
test('Mapping toolbar and inspector edit real bounds, duplication, sequential mapping and safe empty selection',async()=>{
 const {engine,store,presenter,close}=setup();try{presenter.go('mapping');let model=presenter.buildModel();assert.equal(model.mappingSummary,'23 zones · 3 groups');
 await model.insp.rows.find(r=>r.label==='Root').field.onChange(61);assert.equal(engine.getPatch().rules.find(r=>r.id==='k60f').root,61);await store.undo();assert.equal(engine.getPatch().rules.find(r=>r.id==='k60f').root,60);
 await presenter.buildModel().duplicateZone();assert.equal(engine.getPatch().rules.length,25);await store.undo();assert.equal(engine.getPatch().rules.length,23);
 await presenter.buildModel().mapSequential();assert.ok(engine.getPatch().rules.filter(r=>r.group==='keys').every(r=>r.noteLo===r.noteHi&&r.root===r.noteLo));await store.undo();
 await presenter.buildModel().mapVelocity();assert.equal(engine.getPatch().selectors.k60.policy,'velocity');assert.equal(presenter.buildModel().selectorId,'k60');
 await engine.setRules([]);presenter.go('mapping');assert.equal(presenter.buildModel().noZone,true);assert.equal(presenter.buildModel().insp.title,'Key map');
 }finally{close();}
});
test('Picked modulator inspector edits its own route; selected slice Space auditions its actual region',async()=>{
 const {engine,presenter,close}=setup();try{presenter.go('mod');presenter.setState({modInsp:true,pickedMod:'LFO 2'});const model=presenter.buildModel();assert.match(model.insp.title,/LFO 2/);await model.insp.rows.find(r=>r.label==='Amount').field.onChange(50);assert.equal(engine.getPatch().routes.find(r=>r.id==='route-3').amount,.5);assert.equal(engine.getPatch().routes.find(r=>r.id==='route-1').amount,.42);
 presenter.go('slices');presenter.setState({sliceSelection:['slice-6']});presenter.buildModel();presenter.auditionSelection();const call=engine.log.filter(e=>e.kind==='call'&&e.name==='audition').at(-1);assert.equal(call.args[0],'amen');assert.equal(call.args[1].start,.75);
 }finally{close();}
});
test('Source assignment creates usable regions and sound rules; candidate file imports join the specified selector and undo preserves it',async()=>{
 const {engine,store,close}=setup();try{const node=await engine.addNode('drums','sound');await store.operation('Source','assignSource',node,'hard-a');const patch=engine.getPatch();assert.equal(patch.rules.find(r=>r.target===node).source,node);assert.equal(patch.selectors[node].candidates[0].id,'hard-a');engine.noteOn(60,100);assert.ok(engine.telemetry.playheads.length);
 const count=patch.selectors['snare-hard'].candidates.length;await store.operation('Import candidate','importSamples',[{name:'extra.wav'}],{target:'snare-hard'});assert.equal(engine.getPatch().selectors['snare-hard'].candidates.length,count+1);await store.undo();assert.equal(engine.getPatch().selectors['snare-hard'].candidates.length,count);assert.equal(engine.getPatch().assets.some(a=>a.name==='extra.wav'),false);
 }finally{close();}
});
test('Grid and Even slicing produce distinct accepted boundaries, retain shared Slice history and undo restores its count',async()=>{
 const {engine,store,presenter,close}=setup();try{presenter.go('slices');presenter.setState({sliceCount:5,sliceDivision:'grid'});await presenter.buildModel().divideSlices();const grid=engine.getPatch().regions.filter(r=>r.kind==='slice').map(r=>r.start);assert.equal(engine.getPatch().history.find(op=>op.id==='amen-slices').config.count,5);
 await store.undo();assert.equal(engine.getPatch().history.find(op=>op.id==='amen-slices').config.count,8);presenter.setState({sliceDivision:'even'});await presenter.buildModel().divideSlices();const even=engine.getPatch().regions.filter(r=>r.kind==='slice').map(r=>r.start);assert.deepEqual(even,[0,.2,.4,.6,.8]);assert.notDeepEqual(grid,even);
 }finally{close();}
});
test('Cancelled parameter gesture restores its starting value and removes its command; rendered peak sources preserve original assets',async()=>{
 const {engine,store,close}=setup();try{store.beginGesture('Cutoff');store.changeParam('Cutoff',.8);await store.cancelGesture();assert.equal(engine.getPatch().params.Cutoff.value,.42);assert.equal(store.undoCount,0);
 const original=engine.getPeaks('k60f',100);assert.notDeepEqual(Array.from(original),Array.from(engine.getPeaks('hard-a',100)));const id=await store.operation('Collapse','collapseHistory','trim');assert.ok(engine.getPeaks(id,100).length===100);assert.equal(engine.getPatch().assets.some(a=>a.id==='k60f'),true);assert.ok(engine.log.some(e=>e.kind==='result'&&e.name==='collapseHistory'));
 }finally{close();}
});
