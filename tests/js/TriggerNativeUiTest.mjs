import assert from 'node:assert/strict';
import {test} from 'node:test';
import {importActions,selectNativeSamples} from '../../web/trigger/shell/import-actions.mjs';
import {NativeEngine} from '../../web/trigger/engine/native-engine.mjs';
import {backend} from '../../web/trigger/tests/native-fixture.mjs';
import {Store} from '../../web/trigger/engine/store.mjs';
import {ShellPresenter} from '../../web/trigger/shell/ShellPresenter.mjs';

test('native source metadata does not display an invented zero-bit format',async t=>{
  const engine=await new NativeEngine({backend:backend(),pollMs:0,heartbeatMs:0}).initialize();
  const store=new Store(engine);t.after(()=>{store.close();engine.close();});
  store.select({st:'sample',zoneSel:engine.getPatch().rules[0].id});
  const model=new ShellPresenter(store).buildModel();
  assert.ok(model.meta[0].v.includes('48 kHz'));
  assert.equal(model.meta[0].v.includes('0-bit'),false);
});

test('native sample choice preserves decoded file metadata through the mapping dialog',async()=>{
  const file={name:'key_C4.wav',uri:'/samples/key_C4.wav',duration:2,sampleRate:48000,bits:24,channels:2};
  let imported;
  const presenter={state:{},props:{store:{engine:{chooseSamples:async()=>[file]}}},
    setState(delta){Object.assign(this.state,delta);},
    importFiles(files,options){imported={files,options};},queueImport:importActions.queueImport};
  await selectNativeSamples(presenter,{replace:'region'});
  assert.deepEqual(imported,{files:[file],options:{policy:'stack',replace:'region'}});
});

test('cancelled native chooser does not create an editor operation',async()=>{
  let queued=false;
  const presenter={props:{store:{engine:{chooseSamples:async()=>[]}}},queueImport(){queued=true;},setState(){}};
  await selectNativeSamples(presenter);
  assert.equal(queued,false);
});

test('native chooser failure is visible and preserves the editor patch',async()=>{
  const state={};let queued=false;
  const presenter={state,props:{store:{engine:{chooseSamples:async()=>{throw new Error('Unreadable WAV');}}}},
    queueImport(){queued=true;},setState(delta){Object.assign(state,delta);}};
  await selectNativeSamples(presenter);
  assert.equal(queued,false);assert.equal(state.lastImportError,'Unreadable WAV');
});
