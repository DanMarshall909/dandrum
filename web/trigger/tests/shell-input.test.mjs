import test from 'node:test';
import assert from 'node:assert/strict';
import {JSDOM} from 'jsdom';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
import {installComputerKeyboard} from '../shell/computer-keyboard.mjs';

async function setup(t){
  const previous=globalThis.window,dom=new JSDOM('<body><div id="bar"><span data-vh="l"></span><span data-vh="r"></span><span data-vh="m"></span></div></body>');
  globalThis.window=dom.window;
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine),presenter=new ShellPresenter(store);
  presenter.attach();presenter.go('vel');
  const selectorId=presenter.buildModel().selectorId;
  await store.operation('Set velocity fixture','setSelector',selectorId,{policy:'velocity',hardLo:90,softHi:101});
  const bar=dom.window.document.querySelector('#bar');bar.getBoundingClientRect=()=>({left:0,width:126});
  t.after(()=>{presenter.detach();store.close();engine.close();dom.window.close();if(previous===undefined)delete globalThis.window;else globalThis.window=previous;});
  return {engine,store,presenter,selectorId,dom,
    key:(key,options={},target=dom.window.document.body)=>target.dispatchEvent(new dom.window.KeyboardEvent('keydown',{key,bubbles:true,cancelable:true,...options})),
    begin:kind=>presenter.buildModel().onVelDown({target:bar.querySelector('[data-vh="'+kind+'"]'),currentTarget:bar,clientX:89,preventDefault(){}}),
    move:x=>dom.window.dispatchEvent(new dom.window.MouseEvent('pointermove',{clientX:x})),
    finish:event=>dom.window.dispatchEvent(new dom.window.Event(event)),
    settled:async()=>{for(let attempts=0;store.getSnapshot().busy;attempts++){assert.ok(attempts<50,'selector command must settle');await new Promise(resolve=>setTimeout(resolve,1));}},
  };
}

for(const [kind,x,wanted] of [['l',92,[93,101]],['r',97,[90,98]],['m',94,[95,106]]])test('velocity '+kind+' drag publishes one reversible selector edit on release',async t=>{
  const {engine,store,presenter,selectorId,begin,move,finish,settled}=await setup(t);
  const before=store.undoCount;
  begin(kind);move(x);
  assert.deepEqual([engine.getPatch().selectors[selectorId].hardLo,engine.getPatch().selectors[selectorId].softHi],[90,101],'pointer preview does not publish');
  assert.equal(presenter.buildModel().vXfW,wanted[1]-wanted[0]+1);
  finish('pointerup');await settled();await settled();
  assert.deepEqual([engine.getPatch().selectors[selectorId].hardLo,engine.getPatch().selectors[selectorId].softHi],wanted);
  assert.equal(store.undoCount,before+1);
  move(10);finish('pointerup');await settled();
  assert.equal(store.undoCount,before+1,'released listeners cannot publish again');
  await store.undo();
  assert.deepEqual([engine.getPatch().selectors[selectorId].hardLo,engine.getPatch().selectors[selectorId].softHi],[90,101]);
});

for(const ending of ['pointercancel','detach'])test('velocity drag '+ending+' discards preview without a history edit',async t=>{
  const {engine,store,presenter,selectorId,begin,move,finish,settled}=await setup(t);
  const before=store.undoCount;
  begin('l');move(92);assert.equal(presenter.buildModel().vXfW,9);
  if(ending==='detach')presenter.detach();else finish(ending);
  move(20);finish('pointerup');await settled();
  assert.equal(store.undoCount,before);
  assert.deepEqual([engine.getPatch().selectors[selectorId].hardLo,engine.getPatch().selectors[selectorId].softHi],[90,101]);
  assert.equal(presenter.buildModel().vXfW,12);
});

test('mapping pointer release commits changed ranges once and ignores an unchanged drag',async t=>{
  const {engine,store,presenter,finish,settled}=await setup(t);
  presenter.go('mapping');
  const original=engine.getPatch().rules,before=store.undoCount,zones=presenter.getZones();
  presenter.chZones(zones,'k60f');finish('pointerup');await settled();
  assert.equal(store.undoCount,before);assert.deepEqual(engine.getPatch().rules,original);
  presenter.chZones(zones.map(z=>z.id==='k60f'?{...z,lo:62}:z),'k60f');
  assert.deepEqual(engine.getPatch().rules,original);
  finish('pointerup');await settled();
  assert.equal(engine.getPatch().rules.find(r=>r.id==='k60f').noteLo,62);
  assert.equal(store.undoCount,before+1);
  finish('pointerup');await settled();assert.equal(store.undoCount,before+1);
  await store.undo();assert.deepEqual(engine.getPatch().rules,original);
});

test('editing shortcuts undo and redo accepted changes while text fields and dialogs keep their keys',async t=>{
  const {engine,store,dom,key,settled}=await setup(t);
  await store.operation('Selector weight','setCandidate','snare-hard','hard-a',{weight:9});
  const accepted=engine.getPatch(),input=dom.window.document.createElement('input'),dialog=dom.window.document.createElement('div');
  dialog.setAttribute('role','dialog');dom.window.document.body.append(input,dialog);
  for(const target of [input,dialog]){key('z',{ctrlKey:true},target);await settled();assert.deepEqual(engine.getPatch(),accepted);}
  key('z',{ctrlKey:true});await settled();assert.equal(engine.getPatch().selectors['snare-hard'].candidates[0].weight,2);
  key('z',{metaKey:true,shiftKey:true});await settled();assert.equal(engine.getPatch().selectors['snare-hard'].candidates[0].weight,9);
  key('z',{metaKey:true});await settled();
  key('y',{ctrlKey:true});await settled();assert.equal(engine.getPatch().selectors['snare-hard'].candidates[0].weight,9);
});

test('mapping shortcuts navigate, copy, paste, duplicate and delete accepted zones',async t=>{
  const {engine,store,presenter,key,settled}=await setup(t);
  presenter.go('mapping');presenter.buildModel();
  const original=engine.getPatch().rules,ids=presenter.getZones().map(z=>z.id),at=ids.indexOf('k60f');
  key('ArrowRight');assert.equal(presenter.state.zoneSel,ids[(at+1)%ids.length]);
  presenter.buildModel();key('ArrowLeft');assert.equal(presenter.state.zoneSel,'k60f');
  key('c',{ctrlKey:true});key('v',{ctrlKey:true,shiftKey:true});await settled();
  assert.ok(engine.getPatch().rules.some(r=>!original.some(o=>o.id===r.id)&&r.source==='k60f'&&r.noteLo===66));
  await store.undo();assert.deepEqual(engine.getPatch().rules,original);
  presenter.buildModel();key('d',{ctrlKey:true});await settled();
  assert.ok(engine.getPatch().rules.length>original.length);await store.undo();
  presenter.setState({zoneSel:'k60f'});key('Delete');await settled();
  assert.equal(engine.getPatch().rules.some(r=>r.id==='k60f'),false);
  await store.undo();assert.deepEqual(engine.getPatch().rules,original);
});

test('layer shortcuts navigate selected candidates and remove only the chosen candidate',async t=>{
  const {engine,store,presenter,key,settled}=await setup(t);
  presenter.go('rr');presenter.setState({selectorTarget:'snare-hard'});
  assert.equal(presenter.buildModel().selectedCandidate,'hard-b');
  key('ArrowLeft');assert.equal(presenter.buildModel().selectedCandidate,'hard-a');
  key('ArrowRight');assert.equal(presenter.buildModel().selectedCandidate,'hard-b');
  key('Delete');await settled();
  assert.deepEqual(engine.getPatch().selectors['snare-hard'].candidates.map(c=>c.id),['hard-a','hard-c']);
  await store.undo();assert.deepEqual(engine.getPatch().selectors['snare-hard'].candidates.map(c=>c.id),['hard-a','hard-b','hard-c']);
});

test('slice shortcuts navigate, add and remove markers through accepted history',async t=>{
  const {engine,store,presenter,key,settled}=await setup(t);
  presenter.go('slices');presenter.buildModel();
  key('ArrowLeft');assert.deepEqual(presenter.state.sliceSelection,['slice-1']);
  presenter.buildModel();key('ArrowRight');assert.deepEqual(presenter.state.sliceSelection,['slice-2']);
  const original=engine.getPatch();presenter.setState({sliceCursor:.3});presenter.buildModel();
  key('m');await settled();assert.ok(engine.getPatch().regions.some(r=>r.kind==='slice'&&r.start===.3));
  await store.undo();assert.deepEqual(engine.getPatch(),original);
  presenter.buildModel();key('Delete');await settled();assert.equal(engine.getPatch().regions.some(r=>r.id==='slice-2'),false);
  await store.undo();assert.deepEqual(engine.getPatch(),original);
});

test('tree and modulation shortcuts change only their selected entities and Escape dismisses mapping menus',async t=>{
  const {engine,store,presenter,dom,key,settled}=await setup(t);
  const node=dom.window.document.createElement('div');node.setAttribute('data-node','snare');dom.window.document.body.append(node);
  const before=engine.getPatch();key('d',{ctrlKey:true},node);await settled();
  assert.ok(engine.getPatch().nodes.length>before.nodes.length);
  await store.undo();assert.deepEqual(engine.getPatch(),before);
  key('Delete',{},node);await settled();assert.equal(engine.getPatch().nodes.some(n=>n.id==='snare'),false);
  await store.undo();assert.deepEqual(engine.getPatch(),before);
  presenter.go('mod');presenter.buildModel();const route=before.routes.find(r=>!r.locked);
  presenter.setState({selectedRoute:route.id});presenter.buildModel();key('Delete');await settled();
  assert.equal(engine.getPatch().routes.some(r=>r.id===route.id),false);
  await store.undo();assert.deepEqual(engine.getPatch(),before);
  presenter.go('mapping');presenter.buildModel();presenter.setState({menu:{note:60}});key('Escape');assert.equal(presenter.state.menu,null);
  presenter.setState({cmenu:{type:'zone'},dmenu:{type:'source'}});key('Escape');
  assert.equal(presenter.state.cmenu,null);assert.equal(presenter.state.dmenu,null);
});

test('installed keyboard auditions once, opens the targeted modulation menu and releases held notes on removal',async t=>{
  const {engine,presenter,dom,key}=await setup(t);
  const uninstall=installComputerKeyboard(presenter.keyboardBindings);
  try{
  const knob=dom.window.document.createElement('div');knob.setAttribute('data-knob','Cutoff');dom.window.document.body.append(knob);
  key('m',{},knob);assert.equal(presenter.state.kmenu.label,'Cutoff');
  key('Escape');assert.equal(presenter.state.kmenu,null);
  presenter.go('slices');presenter.buildModel();
  const before=engine.log.filter(e=>e.kind==='call'&&e.name==='audition').length;
  key(' ');key(' ',{repeat:true});
  assert.equal(engine.log.filter(e=>e.kind==='call'&&e.name==='audition').length,before+1);
  key('a',{shiftKey:true});key('a',{repeat:true});
  assert.deepEqual(engine.log.filter(e=>e.kind==='call'&&e.name==='noteOn').map(e=>e.args),[[48,127]]);
  dom.window.dispatchEvent(new dom.window.KeyboardEvent('keyup',{key:'a'}));
  key('s');uninstall();
  assert.deepEqual(engine.log.filter(e=>e.kind==='call'&&e.name==='noteOff').map(e=>e.args),[[48],[50]]);
  key('a');assert.equal(engine.log.filter(e=>e.kind==='call'&&e.name==='noteOn').length,2);
  }finally{uninstall();}
});
