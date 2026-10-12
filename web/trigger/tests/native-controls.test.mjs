import test from 'node:test';
import assert from 'node:assert/strict';
import {parse} from 'yaml';
import {NativeEngine} from '../engine/native-engine.mjs';
import {nativeControls} from '../engine/native-controls.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
import {backend} from './native-fixture.mjs';

test('every native exposed React knob invokes its actual host parameter with normalized value and generation',async t=>{
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});await engine.initialize();
  const store=new Store(engine),presenter=new ShellPresenter(store);t.after(()=>{store.close();engine.close();});
  store.select({st:'voice',zoneSel:'kit.kick',snapZero:false});
  const model=presenter.buildModel();
  const knobs=model.voiceSecs.flatMap(section=>section.knobs),ids=Object.keys(engine.getPatch().params);
  assert.deepEqual(knobs.map(knob=>knob.key).sort(),ids.sort());
  for(const [index,knob] of knobs.entries()){
    const value=.2+index*.01;await knob.set(value);
    assert.deepEqual(bridge.calls.filter(call=>call[0]==='setParameter').at(-1),['setParameter',knob.key,value,7]);
    assert.equal(store.getSnapshot().patch.params[knob.key].value,value);
  }
  const selectedSample=model.sampleKnobGroups.slice(1).flatMap(group=>group.knobs);
  assert.ok(selectedSample.length);assert.ok(selectedSample.every(knob=>ids.includes(knob.key)));
  assert.equal(knobs.some(knob=>['Fine','Bend','Key track','Cutoff'].includes(knob.key)),false);
  store.select({st:'sample'});
  const fields=['start_frame','end_frame','fade_in_ms','fade_out_ms'],values=[.01,.22,.001,.002];
  for(let index=0;index<fields.length;index++){
    const regionKnob=presenter.buildModel().sampleKnobGroups[0].knobs[index];await regionKnob.commit(values[index]);
    const graph=parse(bridge.calls.filter(call=>call[0]==='prepareSampler').at(-1)[1]),region=graph.assets.sample_sources[0].regions.find(region=>region.id==='kick');
    const expected=index<2?Math.round(values[index]*51000):values[index]*(51000/48000)*1000;
    assert.equal(region[fields[index]],expected);
  }
});

test('native knob displayed units and typed values use the actual declared control range',async t=>{
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});await engine.initialize();t.after(()=>engine.close());
  const store=new Store(engine);t.after(()=>store.close());const presenter=new ShellPresenter(store),model=presenter.buildModel();nativeControls(presenter,model);
  const level=model.voiceSecs.flatMap(section=>section.knobs).find(knob=>knob.key==='drums.level');
  assert.equal(level.t,'1');assert.equal(level.parse('2'),.5);assert.equal(level.parse('invalid'),null);
  const pan=model.voiceSecs.flatMap(section=>section.knobs).find(knob=>knob.key==='drums.pan');assert.equal(pan.parse('0'),.5);
});

test('a React knob gesture brackets queued native writes once and preserves the last accepted value',async t=>{
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});await engine.initialize();
  const store=new Store(engine);t.after(()=>{store.close();engine.close();});const presenter=new ShellPresenter(store);
  const knob=presenter.buildModel().voiceSecs.flatMap(section=>section.knobs).find(knob=>knob.key==='drums.level');
  const begin=store.beginGesture(knob.key),write=knob.set(.8),end=store.endGesture(knob.key);await Promise.all([begin,write,end]);
  assert.deepEqual(bridge.calls.filter(call=>['beginGesture','setParameter','endGesture'].includes(call[0])),[
    ['beginGesture','drums.level',7],['setParameter','drums.level',.8,7],['endGesture','drums.level',7],
  ]);assert.equal(engine.getPatch().params['drums.level'].value,.8);
  store.beginGesture('Region start');await store.endGesture('Region start');assert.equal(bridge.calls.filter(call=>call[0]==='beginGesture').length,1);
  const nextBegin=store.beginGesture(knob.key),nextWrite=knob.set(.4),cancel=store.cancelGesture();await Promise.all([nextBegin,nextWrite,cancel]);
  assert.deepEqual(bridge.calls.filter(call=>['beginGesture','setParameter','endGesture'].includes(call[0])).slice(-4),[
    ['beginGesture','drums.level',7],['setParameter','drums.level',.4,7],['setParameter','drums.level',.8,7],['endGesture','drums.level',7],
  ]);assert.equal(engine.getPatch().params['drums.level'].value,.8);
});
