import test from 'node:test';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';

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
