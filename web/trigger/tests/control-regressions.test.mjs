import test from 'node:test';
import {mockSampleAt,zeroCrossings,snapZero} from '../engine/mock-wave.mjs';
import {envelopeValue} from '../shell/envelope-values.mjs';
import assert from 'node:assert/strict';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
import {parameterUnits} from '../shell/parameter-units.mjs';
import {envelopeGeometry} from '../shell/envelope-geometry.mjs';
import {modulatorShape} from '../shell/modulator-shape.mjs';
import {modulatorKnobs} from '../shell/modulator-knobs.mjs';
import {storyProps} from '../catalog/story-props.mjs';
const setup=t=>{const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine),presenter=new ShellPresenter(store);t.after(()=>{store.close();engine.close();});return {engine,store,presenter};};
test('typed parameter values retain their displayed units and canonical centres',()=>{
 for(const [label,value,text] of [['Cutoff',.42,'2.40 kHz'],['Tune',.5,'0 st'],['Fine',.47,'−3 ct'],['Gain',.72,'−1.5 dB'],['Pan',.5,'C'],['Lush Pan',.41,'L18'],['Lush Level',.55,'−6 dB'],['Env amt',.71,'+42%'],['Bend',.17,'±2 st']]){
  const units=parameterUnits(label);assert.equal(units.format(value),text);assert.ok(Math.abs(units.parse(text)-value)<1e-12,label);
 }
 assert.equal(parameterUnits('Cutoff').parse('invalid'),null);assert.equal(parameterUnits('Cutoff').parse('20 kHz'),1);assert.equal(parameterUnits('Tune').parse('-24 st'),0);
});
test('modulator percentage entries remain percentages while time and rate entries use seconds and hertz',t=>{
 const {engine,presenter}=setup(t),env=engine.getPatch().modulators.find(m=>m.id==='Filter envelope'),lfo=engine.getPatch().modulators.find(m=>m.id==='LFO 1');
 assert.equal(modulatorKnobs(presenter,env).find(k=>k.label==='Sustain').parse('50%'),.5);
 assert.equal(modulatorKnobs(presenter,lfo).find(k=>k.label==='Phase').parse('25%'),.25);
 assert.equal(modulatorKnobs(presenter,lfo).find(k=>k.label==='Rate').parse('2.40 Hz'),.12);
 assert.ok(Math.abs(modulatorKnobs(presenter,env).find(k=>k.label==='Attack').parse('0.012 s')-.0012)<1e-12);
});
test('an operation undo preserves other fields within the same module, node, macro and candidate',async t=>{
 const {engine,store}=setup(t);await store.operation('Cutoff','setModuleParam','filter','cutoff',.9);engine.setParam('Reso',.8);await store.undo();assert.equal(engine.getPatch().modules.find(m=>m.id==='filter').params.resonance,.8);assert.equal(engine.getPatch().params.Reso.value,.8);
 await store.operation('Output','setOutput','snare','main');await engine.setSend('snare','rev',.8);await store.undo();assert.equal(engine.getPatch().nodes.find(n=>n.id==='snare').sends.rev,.8);assert.equal(engine.getPatch().nodes.find(n=>n.id==='snare').output,'snare');
 await store.operation('Rename','renameMacro','Tone','Dark');engine.setMacro('Tone',.9);await store.undo();assert.equal(engine.getPatch().macros[0].name,'Tone');assert.equal(engine.getPatch().macros[0].value,.9);
 await store.operation('Weight','setCandidate','snare-hard','hard-a',{weight:10});await engine.setCandidate('snare-hard','hard-b',{weight:7});await store.undo();assert.equal(engine.getPatch().selectors['snare-hard'].candidates[1].weight,7);assert.equal(engine.getPatch().selectors['snare-hard'].candidates[0].weight,2);
});
test('envelope handles coincide with the drawn corners and point edits change the actual shape',async t=>{
 const {engine,presenter}=setup(t);presenter.go('voice');for(const mod of engine.getPatch().modulators.filter(m=>m.kind==='envelope')){const g=envelopeGeometry(mod);for(const h of g.handles){assert.ok(g.points.some(([x,v])=>Math.abs(x-h.x)<1e-12&&Math.abs(v-(1-h.y))<1e-12));assert.ok(Math.abs(modulatorShape(mod,h.x)-(1-h.y))<1e-8);}}
 let env=presenter.buildModel().envs[1];assert.equal(env.config.attack,.001);await env.setShape('multi');env=presenter.buildModel().envs[1];await env.onPoint('point-2',.25,.6);assert.deepEqual(engine.getPatch().modulators.find(m=>m.id===env.id).config.points[2],[.25,.4]);
});
test('Space and source assignment use the displayed default selection, including nested alternates and patch sources',async t=>{
 const {engine,store,presenter}=setup(t);presenter.go('slices');presenter.auditionSelection();assert.equal(engine.log.filter(e=>e.kind==='call'&&e.name==='audition').at(-1).args[1].id,'slice-2');
 presenter.go('vel');presenter.auditionSelection();assert.equal(engine.log.filter(e=>e.kind==='call'&&e.name==='audition').at(-1).args[0],'hard-a');
 assert.equal(presenter.buildModel().srcGroups.find(g=>g.id==='patch').items[0].name,'Lush');await store.operation('Add patch layer','addCandidate','snare-hard','module:lush');assert.equal(engine.getPatch().selectors['snare-hard'].candidates.at(-1).name,'Lush');
 assert.deepEqual(engine.telemetry.selectors.resolve('module:lush',60,100),['module:lush']);await store.undo();assert.equal(engine.getPatch().selectors['snare-hard'].candidates.length,3);
});
test('catalog text, shape and value controls bind to individual stories and model callbacks log without logging render reads',t=>{
 const {presenter}=setup(t),base=presenter.buildModel(),events=[],props=storyProps('EnvelopeEditor',base,{state:'default',label:'Env preview',value:.7,checked:true,shape:'multi'},(name,args)=>events.push({name,args}));
 assert.equal(events.length,0);assert.equal(props.envelope.title,'Env preview');assert.equal(props.envelope.shape,'multi');assert.equal(props.envelope.config.sustain,.7);assert.equal(props.envelope.handles.length,6);
 const macro=storyProps('MacroControl',base,{state:'selected',label:'Colour',value:.8,checked:true,shape:'adsr'},(name,args)=>events.push({name,args}));assert.equal(macro.macro.name,'Colour');assert.equal(macro.macro.v,.8);macro.macro.set(.2);assert.equal(events.at(-1).name,'onChange');
 props.model.srcGroups[0].toggle();assert.match(events.at(-1).name,/srcGroups.*toggle/);assert.equal(presenter.state.openSrc.keys,true);
});
test('zero-crossing snap chooses a signed fixture crossing rather than a rounded time grid',()=>{
  const positions=zeroCrossings('k60f'),position=snapZero('k60f',.55);
  assert.ok(positions.includes(position));assert.ok(Math.abs(position-.55)<.01);
  const frame=Math.floor(position*4096);assert.ok(mockSampleAt('k60f',frame)*mockSampleAt('k60f',frame+1)<=0);
  assert.notEqual(position,Math.round(position*1000)/1000);
});
test('undoing a candidate reorder retains unrelated current candidate values',async t=>{
  const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine);t.after(()=>{store.close();engine.close();});
  await store.operation('Reorder','reorderCandidates','snare-hard',['hard-c','hard-a','hard-b']);
  await engine.setCandidate('snare-hard','hard-a',{weight:9});await store.undo();
  assert.deepEqual(engine.getPatch().selectors['snare-hard'].candidates.map(c=>c.id),['hard-a','hard-b','hard-c']);
  assert.equal(engine.getPatch().selectors['snare-hard'].candidates[0].weight,9);
});
test('envelope millisecond, second, decibel and percentage fields preserve canonical values',()=>{
  const amp={id:'Amp envelope',config:{attack:.012,release:1.2,sustain:.5}},filter={id:'Filter envelope',config:{attack:.001,sustain:.3}},values=[];
  for(const [mod,key,wanted,unit] of [[amp,'attack',12,'ms'],[filter,'attack',1,'ms'],[amp,'release',1.2,'s'],[filter,'sustain',30,'%']]){
    const field=envelopeValue(mod,key,v=>values.push(v));assert.equal(field.value,wanted);assert.equal(field.unit,unit);field.change(wanted);assert.equal(values.at(-1),mod.config[key]);
  }
  const field=envelopeValue(amp,'sustain',v=>values.push(v));assert.equal(field.format(field.value),'-6.0');field.change(field.value);assert.equal(values.at(-1),.5);
  const missing=envelopeValue({id:'Filter envelope',config:{}},'hold',()=>{});assert.equal(missing.value,0);
});
