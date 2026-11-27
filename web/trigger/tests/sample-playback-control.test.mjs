import test from 'node:test';
import assert from 'node:assert/strict';
import {session} from './native-fixture.mjs';
import {patchFromSession,compileNativePatch} from '../engine/native-document.mjs';
import {sampleModel} from '../shell/sample-model.mjs';

test('sample playback control starts a forward loop inside its selected native region',async()=>{
  const initial=session(),patch=patchFromSession(initial),calls=[];
  const store={getSnapshot:()=>({patch}),engine:{capabilities:{zeroCrossings:false}},operation:async(...args)=>calls.push(args)};
  const presenter={props:{store},state:{zoneSel:'kit.kick'},regionDefaults:new Map(),sampleRevision:0,setState:()=>{},nn:()=>''};
  const model={page:'sample',isDrop:false,sampleKnobGroups:[{knobs:[]}]};
  sampleModel(presenter,model);
  await model.setPlayback('loop');
  const [,,,delta]=calls[0],kick=patch.regions.find(region=>region.id==='drums.kick');
  assert.equal(delta.playback,'loop');
  assert.ok(delta.loop.start>=kick.start&&delta.loop.end<=kick.end);
  assert.ok(delta.loop.start<delta.loop.end);
  assert.ok(delta.loop.crossfade<=(delta.loop.end-delta.loop.start)/2);
  Object.assign(kick,delta);
  assert.doesNotThrow(()=>compileNativePatch(patch,initial));
});
