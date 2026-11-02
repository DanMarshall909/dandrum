import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {NativeEngine} from '../engine/native-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
import {backend} from './native-fixture.mjs';

async function setup(t){
  const previous=globalThis.window,dom=new JSDOM('<body><div data-knob="drums.level"><div role="slider" aria-label="level"><span>dial</span></div></div><div role="slider" aria-label="drums.pan"></div><div data-reset="macro|amount"></div></body>');
  globalThis.window=dom.window;
  const bridge=backend(),engine=new NativeEngine({backend:bridge,pollMs:0,heartbeatMs:0});
  await engine.initialize();
  const store=new Store(engine),presenter=new ShellPresenter(store);presenter.attach();
  t.after(async()=>{presenter.detach();await store.close();await engine.close();dom.window.close();if(previous===undefined)delete globalThis.window;else globalThis.window=previous;});
  return {bridge,engine,store,presenter,dom,
    pointer:(target,type,options={})=>target.dispatchEvent(new dom.window.MouseEvent(type,{bubbles:true,button:0,...options})),
    settled:()=>new Promise(setImmediate),
    gestures:()=>bridge.calls.filter(call=>['beginGesture','setParameter','endGesture'].includes(call[0])),
  };
}

test('a pointer on a named control brackets native writes until pointer release',async t=>{
  const {presenter,dom,pointer,settled,gestures}=await setup(t),control=dom.window.document.querySelector('[data-knob]');
  pointer(control,'pointerdown');await settled();
  const knob=presenter.buildModel().voiceSecs.flatMap(section=>section.knobs).find(knob=>knob.key==='drums.level');
  await knob.set(.4);await knob.set(.6);
  assert.deepEqual(gestures(),[['beginGesture','drums.level',7],['setParameter','drums.level',.4,7],['setParameter','drums.level',.6,7]]);
  pointer(control,'pointerup');await settled();
  assert.deepEqual(gestures().at(-1),['endGesture','drums.level',7]);
  pointer(control,'pointerup');await settled();
  assert.equal(gestures().filter(call=>call[0]==='endGesture').length,1);
});

test('standalone sliders keep their public label while unrelated and nonprimary input starts no native gesture',async t=>{
  const {dom,pointer,settled,gestures}=await setup(t),doc=dom.window.document,control=doc.querySelector('[data-knob]');
  for(const [target,options] of [[control,{button:1}],[control,{button:2}],[control,{altKey:true}],[doc.body,{}],[doc.querySelector('[data-reset]'),{}]]){
    pointer(target,'pointerdown',options);pointer(target,'pointerup');
  }
  await settled();assert.deepEqual(gestures(),[]);
  const standalone=doc.querySelector('[aria-label="drums.pan"]');
  pointer(standalone,'pointerdown');pointer(standalone,'pointerup');await settled();
  assert.deepEqual(gestures(),[['beginGesture','drums.pan',7],['endGesture','drums.pan',7]]);
});

test('a nested slider uses its enclosing public parameter identity for the complete drag',async t=>{
  const {presenter,dom,pointer,settled,gestures}=await setup(t),dial=dom.window.document.querySelector('[data-knob] [role="slider"] span');
  pointer(dial,'pointerdown');await settled();
  assert.deepEqual(gestures(),[['beginGesture','drums.level',7]],'begin the public parameter, not its short display label');
  const knob=presenter.buildModel().voiceSecs.flatMap(section=>section.knobs).find(knob=>knob.key==='drums.level');
  await knob.set(.4);await knob.set(.6);
  assert.equal(gestures().some(call=>call[0]==='endGesture'),false,'keep the gesture active while the pointer is held');
  pointer(dial,'pointerup');await settled();
  assert.deepEqual(gestures(),[
    ['beginGesture','drums.level',7],['setParameter','drums.level',.4,7],
    ['setParameter','drums.level',.6,7],['endGesture','drums.level',7],
  ]);
});

for(const event of ['pointercancel','blur'])test(event+' rolls back a native drag and ends its gesture once',async t=>{
  const {engine,presenter,dom,pointer,settled,gestures}=await setup(t),control=dom.window.document.querySelector('[data-knob]');
  const before=engine.getPatch().params['drums.level'].value;
  pointer(control,'pointerdown');await settled();
  const knob=presenter.buildModel().voiceSecs.flatMap(section=>section.knobs).find(knob=>knob.key==='drums.level');
  await knob.set(.4);
  dom.window.dispatchEvent(new dom.window.Event(event));await settled();await settled();
  assert.equal(engine.getPatch().params['drums.level'].value,before,'cancel restores the accepted value');
  assert.deepEqual(gestures(),[
    ['beginGesture','drums.level',7],['setParameter','drums.level',.4,7],
    ['setParameter','drums.level',before,7],['endGesture','drums.level',7],
  ]);
  pointer(control,'pointerup');dom.window.dispatchEvent(new dom.window.Event(event));await settled();
  assert.equal(gestures().filter(call=>call[0]==='endGesture').length,1);
});

test('detaching during a native drag ends once and removes pointer listeners',async t=>{
  const {engine,presenter,dom,pointer,settled,gestures}=await setup(t),control=dom.window.document.querySelector('[data-knob]');
  pointer(control,'pointerdown');await settled();
  const knob=presenter.buildModel().voiceSecs.flatMap(section=>section.knobs).find(knob=>knob.key==='drums.level');
  await knob.set(.4);presenter.detach();await settled();
  assert.equal(engine.getPatch().params['drums.level'].value,.4);
  assert.deepEqual(gestures(),[['beginGesture','drums.level',7],['setParameter','drums.level',.4,7],['endGesture','drums.level',7]]);
  pointer(control,'pointerdown');pointer(control,'pointerup');dom.window.dispatchEvent(new dom.window.Event('blur'));await settled();
  assert.equal(gestures().length,3,'detached controls no longer reach native gestures');
});
