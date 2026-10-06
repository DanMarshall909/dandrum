import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {parameterCommand} from '../engine/commands.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';

const setup=async()=>{
  const engine=new MockEngine({preparationMs:0});await engine.loadPreset('felt-kit');
  let clock=0;const store=new Store(engine,{now:()=>clock});
  return {engine,store,tick:ms=>clock+=ms,close:()=>{store.close();engine.close();}};
};

test('rapid changes merge through 600ms, separate gestures and controls keep their own undo',async()=>{
  const {engine,store,tick,close}=await setup();
  for(const [delay,value] of [[0,.5],[600,.6],[601,.7]]){
    tick(delay);store.dispatch(parameterCommand(engine,'Cutoff',value));
  }
  assert.equal(store.undoCount,2);store.undo();assert.equal(engine.getPatch().params.Cutoff.value,.6);
  store.undo();assert.equal(engine.getPatch().params.Cutoff.value,.42);
  store.redo();store.dispatch(parameterCommand(engine,'Reso',.8));
  assert.equal(store.canRedo,false);store.undo();assert.equal(engine.getPatch().params.Reso.value,.22);
  for(const value of [.3,.9]){
    store.beginGesture('Cutoff');store.dispatch(parameterCommand(engine,'Cutoff',value));store.endGesture('Cutoff');
  }
  assert.equal(store.undoCount,3);store.undo();assert.equal(engine.getPatch().params.Cutoff.value,.3);
  close();
});

test('a failed prepared command leaves history and patch operands available for retry',async()=>{
  const {engine,store,close}=await setup();
  store.dispatch(parameterCommand(engine,'Cutoff',.8));
  let fail=true;
  const command={id:'test-prepare',label:'Prepared change',
    do:async adapter=>{if(fail)throw new Error('Preparation failed');adapter.setParam('Reso',.7);},
    undo:async adapter=>{if(fail)throw new Error('Undo failed');adapter.setParam('Reso',.22);}};
  await assert.rejects(store.dispatch(command),/Preparation failed/);
  assert.equal(store.undoLabel,'Change Cutoff');assert.equal(engine.getPatch().params.Reso.value,.22);
  fail=false;await store.dispatch(command);
  fail=true;await assert.rejects(store.undo(),/Undo failed/);
  assert.equal(store.undoLabel,'Prepared change');assert.equal(store.canRedo,false);
  assert.equal(engine.getPatch().params.Reso.value,.7);
  fail=false;await store.undo();assert.equal(store.redoLabel,'Prepared change');
  await store.redo();assert.equal(engine.getPatch().params.Reso.value,.7);
  close();
});

test('navigation and layout use one editor state without resetting the chosen size',async()=>{
  const {store,close}=await setup();const presenter=new ShellPresenter(store);
  presenter.setState({layout:'exp',browser:true});presenter.go('voice');
  assert.equal(store.getSnapshot().editor.layout,'exp');
  assert.equal(store.getSnapshot().editor.st,'voice');
  assert.equal(presenter.buildModel().W,1600);
  assert.equal(presenter.buildModel().browser,true);
  close();
});

test('successful replacement clears history while a failed load preserves the accepted edited patch',async()=>{
  const {engine,store,close}=await setup();store.changeParam('Cutoff',.63);
  store.select({selection:{node:'snare'},st:'voice'});
  await assert.rejects(store.replacePatch('load','unknown'),/Unknown preset/);
  assert.equal(engine.getPatch().params.Cutoff.value,.63);assert.equal(store.undoLabel,'Change Cutoff');
  assert.equal(store.getSnapshot().editor.selection.node,'snare');assert.equal(store.getSnapshot().phase,'failed');
  await store.replacePatch('reload');assert.equal(engine.getPatch().params.Cutoff.value,.42);
  assert.equal(store.undoCount,0);assert.equal(store.getSnapshot().editor.selection,null);
  await store.replacePatch('new');assert.equal(engine.getPatch().name,'Untitled');assert.equal(store.getSnapshot().editor.st,'empty');
  close();
});
test('cancelling a gesture waits for prepared edits and restores only that gesture; separate prepared gestures do not merge',async t=>{
  const {engine,store,close}=await setup();t.after(close);
  store.beginGesture('Slice gain');
  const first=store.operation('Gain','editSlices',['slice-2'],{gain:-3});
  const second=store.operation('Gain','editSlices',['slice-2'],{gain:-9});
  const cancellation=store.cancelGesture();
  await Promise.all([first,second,cancellation]);
  assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-2').gain,0);assert.equal(store.undoCount,0);
  for(const value of [-3,-9]){store.beginGesture('Slice gain');const edit=store.operation('Gain','editSlices',['slice-2'],{gain:value});store.endGesture('Slice gain');await edit;}
  assert.equal(store.undoCount,2);await store.undo();assert.equal(engine.getPatch().regions.find(r=>r.id==='slice-2').gain,-3);
});

test('parameter gestures queued behind preparation undo and redo their accepted values in order',async t=>{
  const {engine,store,close}=await setup();t.after(close);
  const rename=store.operation('Rename group','renameNode','keys','Piano');
  const edits=[.5,.8].map(value=>{store.beginGesture('Cutoff');const edit=store.changeParam('Cutoff',value);store.endGesture('Cutoff');return edit;});
  await Promise.all([rename,...edits]);
  assert.equal(engine.getPatch().params.Cutoff.value,.8);assert.equal(store.undoCount,3);
  await store.undo();assert.equal(engine.getPatch().params.Cutoff.value,.5);
  await store.undo();assert.equal(engine.getPatch().params.Cutoff.value,.42);
  await store.redo();assert.equal(engine.getPatch().params.Cutoff.value,.5);
  await store.redo();assert.equal(engine.getPatch().params.Cutoff.value,.8);
  await store.undo();assert.equal(engine.getPatch().params.Cutoff.value,.5);
  store.beginGesture('Cutoff');const cancelled=store.changeParam('Cutoff',.9);
  await store.cancelGesture();await cancelled;
  assert.equal(engine.getPatch().params.Cutoff.value,.5);assert.equal(store.undoCount,2);
});
