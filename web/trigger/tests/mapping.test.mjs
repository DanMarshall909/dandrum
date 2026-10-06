import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';

test('mapping deletion, paste and neighbour edits change accepted rules and undo preserves unrelated parameters',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine),ui=new ShellPresenter(store);
  t.after(()=>{store.close();engine.close();});
  assert.equal(ui.getZones().length,23);
  const original=engine.getPatch().rules;
  ui.del(ui.getZones().find(z=>z.id==='k60f'),true);await store.pending;
  assert.equal(engine.getPatch().rules.some(r=>r.id==='k60f'),false);
  engine.setParam('Cutoff',.77);await store.undo();
  assert.deepEqual(engine.getPatch().rules,original);assert.equal(engine.getPatch().params.Cutoff.value,.77);
  ui.copyRange(ui.getZones().find(z=>z.id==='k60f'));assert.equal(ui.state.clip.zones.length,2);
  ui.paste(72,true);await store.pending;
  const added=engine.getPatch().rules.filter(r=>!original.some(o=>o.id===r.id));
  assert.equal(added.length,2);assert.deepEqual(added.map(r=>[r.noteLo,r.noteHi,r.root]),[[72,77,72],[72,77,72]]);
  assert.deepEqual(added.map(r=>r.source),['k60p','k60f']);
  await store.undo();assert.deepEqual(engine.getPatch().rules,original);
  const zones=ui.getZones(),selected=zones.find(z=>z.id==='k60f');
  ui.chZones(zones.map(z=>z.id===selected.id?{...z,lo:62}:z),selected.id);
  assert.equal(ui.getZones().find(z=>z.id==='k55f').hi,61);
  await ui.commit(ui.getZones(),'Move zone edge');ui.snap=null;
  assert.equal(engine.getPatch().rules.find(r=>r.id==='k60f').noteLo,62);
  await store.undo();assert.deepEqual(engine.getPatch().rules,original);
});

test('replace paste trims note and velocity rectangles without removing non-overlapping velocity layers',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine),ui=new ShellPresenter(store);
  t.after(()=>{store.close();engine.close();});
  const original=engine.getPatch().rules;
  ui.setState({clip:{lo:60,hi:61,label:'test',zones:[{...ui.getZones().find(z=>z.id==='k60f'),lo:60,hi:61,velLo:100,velHi:110}]}});
  ui.paste(72,false);await store.pending;
  const rules=engine.getPatch().rules.filter(r=>r.group==='keys'&&r.source==='k72f');
  assert.deepEqual(rules.map(r=>[r.noteLo,r.noteHi,r.velLo,r.velHi]).sort(),[[72,73,81,99],[72,73,111,127],[74,96,81,127]].sort());
  assert.deepEqual(engine.getPatch().rules.find(r=>r.id==='k72p'),original.find(r=>r.id==='k72p'));
  await store.undo();assert.deepEqual(engine.getPatch().rules,original);
});
