import test from 'node:test';
import assert from 'node:assert/strict';
import {readFileSync} from 'node:fs';
import ts from 'typescript';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';

const source=ts.createSourceFile('contract.ts',readFileSync(new URL('../engine/contract.ts',import.meta.url),'utf8'),ts.ScriptTarget.Latest);
const surface=name=>source.statements.find(s=>ts.isInterfaceDeclaration(s)&&s.name.text===name).members.map(m=>m.name.text);
const onlyDeclared=engine=>new Proxy(Object.fromEntries(surface('EngineAdapter').map(name=>[name,engine[name].bind(engine)])),{
  get(target,name){if(name in target)return target[name];throw new Error('Undeclared adapter dependency: '+String(name));},
});

test('the declared adapter alone supports edit, undo, redo and telemetry subscription',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),adapter=onlyDeclared(engine),store=new Store(adapter);
  t.after(()=>{store.close();engine.close();});
  await store.operation('Rename group','renameNode','keys','Piano');
  assert.equal(adapter.getPatch().nodes.find(n=>n.id==='keys').name,'Piano');
  await store.undo();assert.equal(adapter.getPatch().nodes.find(n=>n.id==='keys').name,'Keys');
  await store.redo();assert.equal(adapter.getPatch().nodes.find(n=>n.id==='keys').name,'Piano');
  let notifications=0;const unsubscribe=store.subscribeTelemetry(()=>notifications++);
  store.noteOn(60,110);assert.ok(notifications>0);assert.equal(store.getTelemetry().voices.active,1);
  store.noteOff(60);unsubscribe();
  assert.deepEqual(store.getLog(),[]);store.clearLog();
  assert.throws(()=>store.setMockSwitch('missing',true),/diagnostics.*unavailable/i);
  store.close();assert.equal(engine.telemetry.timer,null);
});

test('mock diagnostics are explicitly injected separately from the adapter',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),adapter=onlyDeclared(engine);
  const diagnostics=Object.fromEntries(surface('MockDiagnostics').map(name=>[name,engine[name].bind(engine)]));
  const store=new Store(adapter,{diagnostics});t.after(()=>{store.close();engine.close();});
  store.setMockSwitch('missing',true);assert.equal(adapter.getPatch().assets.find(a=>a.id==='k60f').state,'missing');
  assert.ok(store.getLog().some(e=>e.name==='setMockSwitch'));store.clearLog();assert.deepEqual(store.getLog(),[]);
});
