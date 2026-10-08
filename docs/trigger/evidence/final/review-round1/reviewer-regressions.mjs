import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import {MockEngine} from '/tmp/dandrum-trigger-shell/web/trigger/engine/mock-engine.mjs';
import {Store} from '/tmp/dandrum-trigger-shell/web/trigger/engine/store.mjs';

test('R1: undo of a queued second gesture restores the first accepted gesture',async t=>{
 const engine=new MockEngine({preset:'felt-kit',preparationMs:20}),store=new Store(engine);
 t.after(()=>{store.close();engine.close();});
 const pending=store.operation('Rename group','renameNode','keys','Piano');
 store.beginGesture('Cutoff');const first=store.changeParam('Cutoff',.5);store.endGesture('Cutoff');
 store.beginGesture('Cutoff');const second=store.changeParam('Cutoff',.8);store.endGesture('Cutoff');
 await Promise.all([pending,first,second]);assert.equal(engine.getPatch().params.Cutoff.value,.8);
 assert.equal(store.undoCount,3);await store.undo();assert.equal(engine.getPatch().params.Cutoff.value,.5);
});
test('R2: undo file replacement restores Source history and leaves no dangling asset ID',async t=>{
 const engine=new MockEngine({preset:'felt-kit',preparationMs:0,loadMs:0}),store=new Store(engine);
 t.after(()=>{store.close();engine.close();});
 await store.operation('Replace sample','importSamples',[{name:'replacement.wav'}],{replace:'k60f'});
 const replacement=engine.getPatch().regions.find(r=>r.id==='k60f').assetId;
 assert.notEqual(replacement,'k60f');await store.undo();const patch=engine.getPatch();
 assert.equal(patch.regions.find(r=>r.id==='k60f').assetId,'k60f');assert.equal(patch.assets.some(a=>a.id===replacement),false);
 assert.equal(patch.history.find(op=>op.id==='src').config.asset,'k60f');
});
test('R3: the declared EngineAdapter surface supports structural undo',async t=>{
 const contract=readFileSync('/tmp/dandrum-trigger-shell/web/trigger/engine/contract.ts','utf8').split('export interface EngineAdapter {')[1];
 const names=Array.from(contract.matchAll(/(?:^|[;}])\s*(\w+)(?:<[^\n]+?>)?\(/g),m=>m[1]);
 const engine=new MockEngine({preset:'felt-kit',preparationMs:0});
 const adapter=Object.fromEntries(names.map(name=>[name,engine[name].bind(engine)]));const store=new Store(adapter);
 t.after(()=>{store.close();engine.close();});
 await store.operation('Rename group','renameNode','keys','Piano');assert.equal(engine.getPatch().nodes.find(n=>n.id==='keys').name,'Piano');
 await store.undo();assert.equal(engine.getPatch().nodes.find(n=>n.id==='keys').name,'Keys');
});
