import test from 'node:test';
import assert from 'node:assert/strict';
import {interpretFilename,mappingAssignments} from '../engine/filename-mapping.mjs';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';

test('explicit pitch, MIDI, dynamic, velocity and RR tokens are interpreted without treating unrelated digits as notes',()=>{
  assert.deepEqual(interpretFilename('felt_C#4_p_rr2.wav'),{name:'felt_C#4_p_rr2.wav',root:61,velocity:48,rr:2,dynamic:'p'});
  assert.equal(interpretFilename('piano_Db4_vel096.wav').root,61);
  assert.equal(interpretFilename('kick_note036_v127_rr03.wav').velocity,127);
  assert.equal(interpretFilename('piano_C-1_pp.wav').root,0);
  assert.equal(interpretFilename('amen_172.wav').root,null);
  const files=['C4_p.wav','C4_f.wav','G4_p.wav','G4_f.wav'].map(name=>({name}));
  const mapped=mappingAssignments(files,{policy:'root-velocity'});
  assert.deepEqual(mapped.map(r=>[r.root,r.noteLo,r.noteHi,r.velLo,r.velHi]),[[60,60,63,1,72],[60,60,63,73,127],[67,64,96,1,72],[67,64,96,73,127]]);
  assert.throws(()=>mappingAssignments(files,{policy:'sequential',startNote:126}),/exceed/);
});

for(const policy of ['sequential','root-velocity','stack','rr'])test('import '+policy+' is one reversible operation and retains file sources',async t=>{
  const engine=new MockEngine({preparationMs:0,loadMs:0}),store=new Store(engine);t.after(()=>{store.close();engine.close();});
  await store.operation('Import samples','importSamples',[{name:'C4_vel40_rr2.wav'},{name:'C4_vel100_rr1.wav'}],{policy,startNote:48});
  const patch=engine.getPatch();assert.equal(store.undoCount,1);assert.equal(patch.assets.length,2);assert.equal(patch.regions.length,2);
  assert.equal(patch.rules.length,policy==='rr'?1:2);
  if(policy==='rr')assert.equal(patch.selectors[patch.rules[0].source].candidates[0].name,'C4_vel100_rr1.wav');
  await store.undo();assert.equal(engine.getPatch().assets.length,0);assert.equal(engine.getPatch().rules.length,0);
  await store.redo();assert.deepEqual(engine.getPatch(),patch);
});

test('import replacement undo and redo keep the asset, region and Source history consistent',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0,loadMs:0}),store=new Store(engine);
  t.after(()=>{store.close();engine.close();});
  const original=engine.getPatch();
  await store.operation('Replace sample','importSamples',[{name:'replacement.wav'}],{replace:'k60f'});
  const replacement=engine.getPatch().regions.find(r=>r.id==='k60f').assetId;
  assert.notEqual(replacement,'k60f');assert.equal(engine.getPatch().history.find(op=>op.id==='src').config.asset,replacement);
  engine.setParam('Reso',.77);await store.undo();const undone=engine.getPatch();
  assert.equal(undone.regions.find(r=>r.id==='k60f').assetId,'k60f');
  assert.equal(undone.assets.some(a=>a.id===replacement),false);assert.deepEqual(undone.history,original.history);
  assert.equal(undone.params.Reso.value,.77);
  await store.redo();const redone=engine.getPatch();
  assert.equal(redone.assets.find(a=>a.id===replacement).name,'replacement.wav');
  assert.equal(redone.regions.find(r=>r.id==='k60f').assetId,replacement);
  assert.equal(redone.history.find(op=>op.id==='src').config.asset,replacement);assert.equal(redone.params.Reso.value,.77);
});
