import test from 'node:test';
import assert from 'node:assert/strict';
import { MockEngine } from '../engine/mock-engine.mjs';
import { Store } from '../engine/store.mjs';
import { parameterCommand } from '../engine/commands.mjs';

test('Empty startup loads the complete Felt Kit behind preparing and ready events', async () => {
  const engine = new MockEngine({ preparationMs: 0 });
  assert.equal(engine.getPatch().name, 'Untitled');
  assert.deepEqual(engine.getPatch().rules, []);
  const phases = [];
  const unsubscribe = engine.on('patch', e => phases.push(e.phase));
  await engine.loadPreset('felt-kit');
  const patch = engine.getPatch();
  assert.equal(patch.name, 'Felt Kit');
  assert.equal(patch.rules.length, 23);
  assert.deepEqual(patch.rules.filter(z => z.group === 'keys').map(z => [z.root, z.velLo, z.velHi]),
    [48,55,60,66,72].flatMap(root => [[root,1,90],[root,81,127]]));
  assert.deepEqual(patch.rules.filter(z => z.noteLo === 38).map(z => [z.velLo,z.velHi]), [[1,95],[96,127]]);
  assert.equal(patch.selectors['snare-hard'].candidates.length, 3);
  assert.equal(patch.nodes.filter(n => n.choke === 1).length, 2);
  assert.deepEqual(patch.regions.filter(r=>r.kind==='slice').map(r=>r.note), [24,25,26,27,28,29,30,31]);
  assert.deepEqual(phases, ['preparing','ready']);
  unsubscribe();
  // Owned JSON snapshots cannot mutate authoritative engine storage.
  patch.rules[0].noteLo = 99;
  assert.equal(engine.getPatch().rules[0].noteLo, 24);
  engine.close();
});

test('a knob gesture is one named reversible command and redo preserves its final value', async () => {
  const engine = new MockEngine({ preparationMs: 0 });
  await engine.loadPreset('felt-kit');
  const store = new Store(engine);
  const initial = engine.getPatch().params.Cutoff.value;
  store.beginGesture('Cutoff');
  store.dispatch(parameterCommand(engine, 'Cutoff', .55));
  store.dispatch(parameterCommand(engine, 'Cutoff', .68));
  store.dispatch(parameterCommand(engine, 'Cutoff', .79));
  store.endGesture('Cutoff');
  assert.equal(engine.getPatch().params.Cutoff.value, .79);
  assert.equal(store.undoLabel, 'Change Cutoff');
  assert.equal(store.undoCount, 1);
  store.undo();
  assert.equal(engine.getPatch().params.Cutoff.value, initial);
  assert.equal(store.canUndo, false);
  store.redo();
  assert.equal(engine.getPatch().params.Cutoff.value, .79);
  assert.equal(store.canRedo, false);
  assert.throws(()=>store.dispatch(parameterCommand(engine, 'Cutoff', NaN)), /finite/);
  assert.equal(engine.getPatch().params.Cutoff.value, .79);
  store.close(); engine.close();
});
