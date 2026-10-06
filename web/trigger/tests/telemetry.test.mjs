import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';

test('stack, per-note round robin, candidate mutes and missing fallback produce usable silent voice feedback',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0});t.after(()=>engine.close());
  await engine.setSelector('snare-hard',{policy:'stack'});engine.noteOn(38,110);
  assert.deepEqual(engine.telemetry.active.map(v=>v.layer),['hard-a','hard-b','hard-c']);engine.telemetry.reset();
  await engine.setCandidate('snare-hard','hard-b',{muted:true});engine.noteOn(38,110);
  assert.deepEqual(engine.telemetry.active.map(v=>v.layer),['hard-a','hard-c']);engine.telemetry.reset();
  await engine.setSelector('snare-hard',{policy:'rr'});
  engine.noteOn(38,110);engine.noteOn(38,110);
  assert.equal(engine.telemetry.selectorPos['snare-hard'].last,'hard-c');
  assert.deepEqual(engine.telemetry.selectors.resolve('snare-hard',40,110),['hard-a']);
  engine.telemetry.reset();engine.setMockSwitch('missing',true);engine.noteOn(60,110);
  assert.deepEqual(engine.telemetry.active.map(v=>v.layer),['k60p']);
});

test('replacement resets active feedback and continues updates outside the editor store',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine);t.after(()=>{store.close();engine.close();});
  const unsubscribe=store.subscribeTelemetry(()=>{});t.after(unsubscribe);
  engine.noteOn(60,110);assert.equal(store.getTelemetry().voices.active,1);
  await store.replacePatch('new');assert.equal(store.getTelemetry().voices.active,0);assert.equal(store.getTelemetry().notes.length,0);
  assert.ok(engine.telemetry.timer);const editorSnapshot=store.getSnapshot();engine.telemetry.tick(0);assert.equal(store.getSnapshot(),editorSnapshot);
});
