import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
const setup=()=>{const engine=new MockEngine({preset:'felt-kit',preparationMs:0,analysisMs:0}),store=new Store(engine);return {engine,store,close:()=>{store.close();engine.close();}};};
test('slice marker, division, mapping and bulk properties publish atomically and undo restores regions and notes',async()=>{
  const {engine,store,close}=setup();try{
    await store.operation('Move marker','moveSliceBoundary','slice-2',.27);
    assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-1').end,.27);assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-2').start,.27);
    await store.undo();assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-1').end,.25);
    const original=engine.getPatch();await store.operation('Divide','setSlices','amen',[0,.25,.5,.75],{startNote:60});
    const slices=engine.getPatch().regions.filter(r=>r.kind==='slice');assert.deepEqual(slices.map(r=>[r.start,r.end,r.note]),[[0,.25,60],[.25,.5,61],[.5,.75,62],[.75,1,63]]);
    assert.deepEqual(engine.getPatch().rules.filter(r=>r.group==='break').map(r=>r.noteLo),[60,61,62,63]);
    await store.undo();assert.deepEqual(engine.getPatch().regions,original.regions);assert.deepEqual(engine.getPatch().rules,original.rules);
    await store.operation('Map','mapSlices','amen',36);assert.equal(engine.getPatch().rules.find(r=>r.source==='slice-2').noteLo,38);
    await store.operation('Bulk gain','editSlices',['slice-2','slice-6'],{gain:-9});assert.deepEqual(engine.getPatch().regions.filter(r=>['slice-2','slice-6'].includes(r.id)).map(r=>r.gain),[-9,-9]);
    await store.undo();assert.deepEqual(engine.getPatch().regions.filter(r=>['slice-2','slice-6'].includes(r.id)).map(r=>r.gain),[0,0]);
    const before=engine.getPatch();await assert.rejects(engine.setSlices('amen',[0,.00001]),/close/);assert.deepEqual(engine.getPatch(),before);
    await assert.rejects(engine.mapSlices('amen',125),/MIDI/);assert.deepEqual(engine.getPatch(),before);
  }finally{close();}
});
test('history bypass changes only its owning region and collapse retains the original source with reversible operands',async()=>{
  const {engine,store,close}=setup();try{
    const original=engine.getPatch();await store.operation('Bypass trim','setHistory','trim',{on:false});
    assert.equal(engine.getPatch().regions.find(r=>r.id==='k60f').start,0);assert.equal(engine.getPatch().regions.find(r=>r.id==='k60p').start,.012);
    await store.undo();assert.deepEqual(engine.getPatch().regions,original.regions);
    await store.operation('Fade config','setHistory','fade',{config:{in:.01,out:.02}});assert.equal(engine.getPatch().regions.find(r=>r.id==='k60f').fades.in,.01);
    await store.undo();assert.equal(engine.getPatch().regions.find(r=>r.id==='k60f').fades.in,.004);
    const id=await store.operation('Collapse','collapseHistory','trim');assert.notEqual(id,'k60f');
    assert.equal(engine.getPatch().assets.some(a=>a.id==='k60f'),true);assert.equal(engine.getPatch().regions.find(r=>r.id==='k60f').assetId,id);
    await store.undo();assert.deepEqual(engine.getPatch().assets,original.assets);assert.deepEqual(engine.getPatch().regions,original.regions);assert.deepEqual(engine.getPatch().history,original.history);
    await assert.rejects(engine.setHistory('src',{on:false}),/Source/);await assert.rejects(engine.removeHistory('src'),/Source/);
  }finally{close();}
});
test('Slice history owns marker counts, preserves manual identities on bypass, and reverses count edits and deletion',async t=>{
  const {engine,store,close}=setup();t.after(close);
  await store.operation('Split','splitSlice','slice-2',.28);
  assert.equal(engine.getPatch().history.find(op=>op.id==='amen-slices').config.count,9);
  const before=engine.getPatch(),slices=before.regions.filter(r=>r.kind==='slice');
  await store.operation('Bypass','setHistory','amen-slices',{on:false});
  assert.equal(engine.getPatch().regions.filter(r=>r.kind==='slice').length,0);
  assert.equal(engine.getPatch().rules.some(r=>r.group==='break'),false);
  await store.operation('Enable','setHistory','amen-slices',{on:true});
  assert.deepEqual(engine.getPatch().regions.filter(r=>r.kind==='slice'),slices);
  await store.operation('Count','setHistory','amen-slices',{config:{count:4,method:'even'}});
  assert.deepEqual(engine.getPatch().regions.filter(r=>r.kind==='slice').map(r=>[r.start,r.end]),[[0,.25],[.25,.5],[.5,.75],[.75,1]]);
  await store.undo();assert.deepEqual(engine.getPatch().regions.filter(r=>r.kind==='slice'),slices);
  const beforeSplitUndo=engine.getPatch().history.find(op=>op.id==='amen-slices').config.count;
  assert.equal(beforeSplitUndo,9);
  await store.operation('Delete operation','removeHistory','amen-slices');
  assert.equal(engine.getPatch().regions.filter(r=>r.kind==='slice').length,0);
  await store.undo();assert.deepEqual(engine.getPatch().regions.filter(r=>r.kind==='slice'),slices);
});
test('initial slice bypass preserves the supplied nonuniform regions; edits and collapse retain coherent operation operands',async t=>{
  const {engine,store,close}=setup();t.after(close);const before=engine.getPatch();
  await store.operation('Bypass initial slices','setHistory','amen-slices',{on:false});await store.undo();
  assert.deepEqual(engine.getPatch().regions,before.regions);assert.deepEqual(engine.getPatch().rules,before.rules);assert.deepEqual(engine.getPatch().history,before.history);
  await store.operation('Split','splitSlice','slice-2',.28);await store.undo();
  assert.equal(engine.getPatch().history.find(op=>op.id==='amen-slices').config.count,8);
  await store.operation('Manual gain','editSlices',['slice-2'],{gain:-9});await store.undo();
  await engine.setHistory('amen-slices',{on:false});await engine.setHistory('amen-slices',{on:true});
  assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-2').gain,0);
  const baked=await store.operation('Collapse slicing','collapseHistory','amen-slices');
  assert.equal(engine.getPatch().regions.filter(r=>r.kind==='slice').every(r=>r.assetId===baked),true);
  assert.equal(engine.getPatch().history.some(op=>op.id==='amen-slices'),false);
  await store.undo();assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-2').assetId,'amen');
});
test('Slice operation regenerates from distinct grid/transient markers and rejects unsupported density without publishing',async t=>{
  const {engine,close}=setup();t.after(close);
  await engine.setHistory('amen-slices',{config:{count:4,method:'transients'}});
  assert.deepEqual(engine.getPatch().regions.filter(r=>r.kind==='slice').map(r=>r.start),[0,.1875,.4375,.6875]);
  await engine.setHistory('amen-slices',{config:{count:4,method:'grid'}});
  const starts=engine.getPatch().regions.filter(r=>r.kind==='slice').map(r=>r.start);
  assert.ok(Math.abs(starts[1]-.25)<.001);
  const before=engine.getPatch();await assert.rejects(engine.setHistory('amen-slices',{config:{count:64,method:'grid'}}),/grid/);assert.deepEqual(engine.getPatch(),before);
  await assert.rejects(engine.setHistory('amen-slices',{config:{count:16,method:'transients'}}),/15 transients/);assert.deepEqual(engine.getPatch(),before);
});
test('newly imported slicing creates an owning source stack and count edits reject invalid mapping atomically',async t=>{
  const {engine,store,close}=setup();t.after(close);
  const [asset]=await engine.importAssets([{name:'new.wav',duration:1}]);
  await store.operation('Slice import','setSlices',asset.id,[0,.5],{group:'break',startNote:126});
  const before=engine.getPatch(),op=before.history.find(op=>op.regionId===asset.id&&op.name==='Slice');
  assert.equal(op.config.count,2);assert.equal(before.history.filter(op=>op.regionId===asset.id&&op.locked).length,1);
  await assert.rejects(engine.setHistory(op.id,{config:{count:4,method:'even'}}),/MIDI/);
  assert.deepEqual(engine.getPatch(),before);
});
