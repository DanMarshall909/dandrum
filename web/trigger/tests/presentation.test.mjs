import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
import {NativeEngine} from '../engine/native-engine.mjs';
import {backend} from './native-fixture.mjs';

test('a visible Cutoff edit enters the command history and the display follows undo and redo',async()=>{
  const engine=new MockEngine({preparationMs:0});await engine.loadPreset('felt-kit');
  const store=new Store(engine);const presenter=new ShellPresenter(store);
  presenter.go('voice');
  const cutoff=()=>presenter.buildModel().voiceSecs.find(s=>s.title==='Filter').knobs.find(k=>k.key==='Cutoff');
  assert.equal(cutoff().v,.42);
  cutoff().set(.57);
  assert.equal(store.undoLabel,'Change Cutoff');
  assert.equal(cutoff().v,.57);
  presenter.go('mod');
  store.undo();
  assert.equal(cutoff().v,.42);
  assert.equal(presenter.buildModel().noRedo,false);
  store.redo();
  assert.equal(cutoff().v,.57);
  store.close();engine.close();
});

test('native frames fit the host viewport at readable size while browser layout dimensions remain exact',t=>{
  const previous=globalThis.window;t.after(()=>{if(previous===undefined)delete globalThis.window;else globalThis.window=previous;});
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine),presenter=new ShellPresenter(store);t.after(()=>{store.close();engine.close();});
  engine.native=true;globalThis.window={innerWidth:1180,innerHeight:860};
  for(const layout of ['min','default','exp']){store.select({layout});const model=presenter.buildModel();assert.ok(model.W<=1180);assert.ok(model.H<=860);assert.equal(model.scale,1);assert.equal(model.backend,'native');assert.doesNotMatch(model.sizeText,/preview/);}
  globalThis.window={innerWidth:820,innerHeight:560};store.select({layout:'default'});const narrow=presenter.buildModel();assert.equal(narrow.W,820);assert.equal(narrow.H,560);assert.equal(narrow.isMin,true);
  engine.native=false;
  for(const [layout,width,height] of [['min',820,560],['default',1200,800],['exp',1600,1000]]){store.select({layout});const model=presenter.buildModel();assert.equal(model.W,width);assert.equal(model.H,height);assert.equal(model.backend,'mock');}
});

test('native size choices resize the host and reopening restores the selected full layout',async t=>{
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});await engine.initialize();
  const store=new Store(engine),presenter=new ShellPresenter(store);t.after(()=>{store.close();engine.close();});
  bridge.functions.resizeSamplerEditor=()=>({status:'accepted',width:820,height:560});
  for(const layout of ['min','default','exp'])await presenter.buildModel().setLayout(layout);
  await presenter.buildModel().togglePerf();await presenter.buildModel().togglePerf();
  assert.deepEqual(bridge.calls.filter(call=>call[0]==='resizeSamplerEditor'),[
    ['resizeSamplerEditor','min'],['resizeSamplerEditor','default'],['resizeSamplerEditor','exp'],['resizeSamplerEditor','perf'],['resizeSamplerEditor','exp'],
  ]);assert.equal(store.getSnapshot().editor.layout,'exp');assert.equal(store.getSnapshot().editor.perf,false);
  bridge.functions.resizeSamplerEditor=()=> 'Host editor resize unavailable';
  await presenter.buildModel().setLayout('min');assert.equal(presenter.buildModel().status.kind,'error');assert.match(presenter.buildModel().status.text,/Host editor resize unavailable/);
});


test('host-controlled native sizing preserves editor layout without optimistic changes or resize requests',async t=>{
  const previous=globalThis.window,bridge=backend();
  globalThis.window={__DANDRUM__:{protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend:bridge}};
  t.after(()=>{globalThis.window=previous;});
  const engine=new NativeEngine({heartbeatMs:0});await engine.initialize();const store=new Store(engine),presenter=new ShellPresenter(store);t.after(()=>{store.close();engine.close();});
  const before=store.getSnapshot().editor,model=presenter.buildModel();
  assert.equal(model.hostControlledSize,true);assert.equal(model.meteringAvailable,false);assert.equal(model.waveformAvailable,false);
  await model.setLayout('exp');await model.togglePerf();
  assert.equal(store.getSnapshot().editor.layout,before.layout);assert.equal(store.getSnapshot().editor.perf,before.perf);
  assert.deepEqual(bridge.calls.map(call=>call[0]),['negotiateEditorProtocol','getSamplerSession']);
  assert.match(model.backendDetail,/key mapping.*undo.*public controls/i);assert.match(model.backendDetail,/meters.*waveforms.*unavailable/i);
});
