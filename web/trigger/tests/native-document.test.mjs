import test from 'node:test';
import assert from 'node:assert/strict';
import {parse} from 'yaml';
import {session} from './native-fixture.mjs';
import {patchFromSession,compileNativePatch} from '../engine/native-document.mjs';

test('native document projects exact file paths, real frame geometry, public controls and round robin candidates',()=>{
  const patch=patchFromSession(session());
  assert.equal(patch.name,'Advanced Drum Kit');assert.equal(patch.assets[0].uri,'/repo/examples/patches/assets/advanced-drums.wav');
  assert.equal(patch.assets[0].duration,51000/48000);assert.equal(patch.regions.find(r=>r.id==='drums.snare_soft').start,12000/51000);
  assert.equal(patch.params['drums.level'].value,.25);assert.equal(patch.params['drums.level'].max,1);
  assert.equal(patch.selectors['kit.hard_snare'].policy,'rr');assert.equal(patch.selectors['kit.hard_snare'].candidates.length,2);
  const grouped=patch.rules.find(r=>r.source==='kit.hard_snare');assert.equal(grouped.velLo,64);
  assert.equal(grouped.nativeMapId,'kit');assert.equal(grouped.nativeZoneId,'snare_hard_a');
  assert.deepEqual(patch.selectors['kit.hard_snare'].candidates.map(candidate=>candidate.nativeZoneId),['snare_hard_a','snare_hard_b']);
  assert.equal(patch.voicePolicy.limit,8);assert.equal(patch.modules.length,1);
});

test('compiler preserves real graph controls and metadata while applying regions, map candidates and bounded polyphony',()=>{
  const initial=session(),patch=patchFromSession(initial),region=patch.regions.find(r=>r.id==='drums.kick');
  Object.assign(region,{start:100/51000,end:11000/51000,reverse:true,fades:{in:.01/patch.assets[0].duration,out:.02/patch.assets[0].duration},root:36});
  patch.voicePolicy.limit=12;patch.params['drums.level'].value=.5;
  patch.selectors['kit.hard_snare'].candidates[1].muted=true;
  const graph=parse(compileNativePatch(patch,initial));
  assert.equal(graph.assets.sample_sources[0].path,'assets/advanced-drums.wav');
  const compiled=graph.assets.sample_sources[0].regions.find(r=>r.id==='kick');
  assert.deepEqual([compiled.start_frame,compiled.end_frame,compiled.fade_in_ms,compiled.fade_out_ms],[100,11000,10,20]);
  assert.equal(compiled.reverse,true);assert.equal(graph.modules.find(m=>m.type==='sample_map_player').static.max_voices,12);
  assert.equal(graph.assets.sample_maps[0].zones.filter(z=>z.round_robin_group==='kit.hard_snare').length,1);
  assert.equal(graph.ports.find(p=>p.name==='level').default,2);assert.equal(graph.preset_surface.parameters[0].name,'drums.pitch_ratio');
});

test('compiler refuses unsupported DSP, synthetic assets, cyclic selectors and simulated history',()=>{
  for(const mutate of [p=>p.modules.push({id:'filter',type:'Filter',params:{}}),p=>p.assets[0].uri='mock://fake.wav',p=>p.history.push({id:'normalize',name:'Normalize',on:true,config:{}}),p=>p.voicePolicy.mode='legato',p=>p.selectors['kit.hard_snare'].candidates[0].id='kit.hard_snare']){
    const patch=patchFromSession(session());mutate(patch);assert.throws(()=>compileNativePatch(patch,session()),/unsupported|native|cycle/i);
  }
});

test('compiler refuses overlapping stacked rules, unsupported loop modes and unmapped native controls',()=>{
  for(const mutate of [
    p=>p.rules.push({...p.rules.find(rule=>rule.id==='kit.kick'),id:'stacked',source:'drums.snare_soft'}),
    p=>Object.assign(p.regions[0],{playback:'pp',loop:{start:0,end:.1,crossfade:0}}),
    p=>p.params.Cutoff={id:'Cutoff',name:'Cutoff',value:.5},
    p=>Object.assign(p.selectors['kit.hard_snare'].candidates[0],{output:'aux'}),
  ]){const patch=patchFromSession(session());mutate(patch);assert.throws(()=>compileNativePatch(patch,session()),/unsupported/i);}
});

test('forward loop and pitch/gain candidate edits retain actual units and weighted selection',()=>{
  const initial=session(),patch=patchFromSession(initial),region=patch.regions[0];
  region.playback='loop';region.loop={start:1000/51000,end:10000/51000,crossfade:.001/patch.assets[0].duration};
  const selector=patch.selectors['kit.hard_snare'];selector.policy='weighted';selector.candidates[0].gain=-6;selector.candidates[0].pan=-.5;selector.candidates[0].tune=12;selector.candidates[0].weight=3;
  patch.selectors['kit.open_hats'].policy='weighted';
  const graph=parse(compileNativePatch(patch,initial));assert.deepEqual(graph.assets.sample_sources[0].regions[0].loop,{mode:'forward',start_frame:1000,end_frame:10000,crossfade_ms:1});
  assert.equal(graph.assets.sample_maps[0].selection_mode,'random_weighted');const candidate=graph.assets.sample_maps[0].zones.find(zone=>zone.round_robin_group==='kit.hard_snare');
  assert.deepEqual([candidate.gain_db,candidate.pan,candidate.pitch_semitones,candidate.weight],[-6,-.5,12,3]);
});

test('mapping roots compile to distinct playable regions while unchanged drums retain native no-root semantics',()=>{
  const initial=session(),patch=patchFromSession(initial),defaultGraph=parse(compileNativePatch(patch,initial));
  assert.equal(defaultGraph.assets.sample_sources[0].regions.find(region=>region.id==='kick').root_note,undefined);
  const kick=patch.rules.find(rule=>rule.id==='kit.kick');kick.root=48;
  patch.rules.push({...kick,id:'kick-second-root',noteLo:60,noteHi:60,root:60});
  const graph=parse(compileNativePatch(patch,initial)),source=graph.assets.sample_sources[0],zones=graph.assets.sample_maps[0].zones;
  const mapped=id=>source.regions.find(region=>region.id===zones.find(zone=>zone.id===id).region.split('.')[1]);
  assert.equal(mapped('kick').root_note,48);assert.equal(mapped('kick-second-root').root_note,60);
  assert.deepEqual([mapped('kick').start_frame,mapped('kick').end_frame],[0,12000]);
  assert.notEqual(zones.find(zone=>zone.id==='kick').region,zones.find(zone=>zone.id==='kick-second-root').region);
  kick.root=36;patch.rules=patch.rules.filter(rule=>rule.id!=='kick-second-root');
  const restored=parse(compileNativePatch(patch,initial));assert.equal(restored.assets.sample_sources[0].regions.find(region=>region.id==='kick').root_note,undefined);
});

test('native round robin projection rejects candidates whose bounds cannot be represented by one rule',()=>{
  const initial=session();initial.document.maps[0].zones.find(zone=>zone.id==='snare_hard_b').velocityLow=80;
  assert.throws(()=>patchFromSession(initial),/unsupported.*round robin|round robin.*unsupported/i);
});

test('native compiler rejects nested selection policies instead of flattening their audible behavior',()=>{
  const initial=session(),patch=patchFromSession(initial);
  patch.selectors.outer={policy:'rr',candidates:[{id:'kit.hard_snare',weight:1,muted:false,solo:false}]};
  patch.rules.find(rule=>rule.source==='kit.hard_snare').source='outer';
  assert.throws(()=>compileNativePatch(patch,initial),/nested.*unsupported|unsupported.*nested/i);
});

test('native graph sample paths are canonical authoring-root relatives and outside-root files are rejected',()=>{
  const initial=session(),patch=patchFromSession(initial),graph=parse(compileNativePatch(patch,initial));
  assert.equal(graph.assets.sample_sources[0].path,'assets/advanced-drums.wav');
  patch.assets[0].uri='/repo/examples/patches/.tools/trigger-assets/123/import.wav';
  assert.equal(parse(compileNativePatch(patch,initial)).assets.sample_sources[0].path,'.tools/trigger-assets/123/import.wav');
  patch.assets[0].uri='/samples/outside.wav';assert.throws(()=>compileNativePatch(patch,initial),/authoring root|outside/);
  patch.assets[0].uri='/repo/examples/patches/../outside.wav';assert.throws(()=>compileNativePatch(patch,initial),/authoring root|canonical|outside/);
});

test('empty native patch compiles a real silent mixer graph without an invalid empty sample map',()=>{
  const initial=session(),patch=patchFromSession(initial);patch.assets=[];patch.regions=[];patch.rules=[];patch.selectors={};patch.params={};
  const graph=parse(compileNativePatch(patch,initial));
  assert.deepEqual(graph.modules,[{id:'silence',type:'audio_mixer',static:{channels:2}}]);
  assert.equal(graph.assets,undefined);assert.equal(graph.preset_surface,undefined);
  assert.deepEqual(graph.ports,[{name:'master',direction:'output',signal:'audio',channels:2,maps_from:'silence.mix'}]);
});
