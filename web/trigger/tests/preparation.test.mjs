import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';

test('preparation publishes its phase while accepted data remains unchanged until ready',async()=>{
  const engine=new MockEngine({preparationMs:5});const store=new Store(engine);
  const phases=[];const stop=store.subscribe(()=>phases.push(store.getSnapshot().phase));
  const pending=engine.loadPreset('felt-kit');
  assert.equal(store.getSnapshot().phase,'preparing');
  assert.equal(store.getSnapshot().patch.name,'Untitled');
  await pending;
  assert.equal(store.getSnapshot().phase,'ready');
  assert.equal(store.getSnapshot().patch.name,'Felt Kit');
  assert.deepEqual(phases,['preparing','ready']);
  stop();store.close();engine.close();
});

test('closing during preparation cancels late publication and command history',async()=>{
  const engine=new MockEngine({preparationMs:20});const store=new Store(engine);
  let notifications=0;store.subscribe(()=>notifications++);
  const pending=store.dispatch({id:'load',label:'Load',
    do:adapter=>adapter.loadPreset('felt-kit'),undo:adapter=>adapter.loadPreset('empty')});
  store.close();engine.close();const before=notifications;
  await assert.rejects(pending,/closed/);
  assert.equal(engine.getPatch().name,'Untitled');assert.equal(store.undoCount,0);
  assert.equal(notifications,before);
});

test('overlapping preparations publish accepted patches in request order with paired phases',async()=>{
  const engine=new MockEngine({preparationMs:0});const events=[];
  engine.on('patch',event=>events.push([event.phase,event.patch?.name??null]));
  const felt=engine.loadPreset('felt-kit'),empty=engine.loadPreset('empty');
  await Promise.all([felt,empty]);
  assert.deepEqual(events,[['preparing',null],['ready','Felt Kit'],['preparing',null],['ready','Untitled']]);
  assert.equal(engine.getPatch().name,'Untitled');engine.close();
});
