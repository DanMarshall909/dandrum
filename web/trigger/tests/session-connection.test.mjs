import test from 'node:test';
import assert from 'node:assert/strict';
import {parseSessionConnection,nextDecimalId} from '../engine/session-connection.mjs';
import {patchFromSession} from '../engine/native-document.mjs';
import {session} from './native-fixture.mjs';

const valid={
  version:1,
  instanceId:'18446744073709551615',
  attachmentId:'9007199254740993',
  revision:'0',
  nextRequestId:'1',
  limits:{attachments:16,history:100,outcomes:128,activeRequests:1},
  history:{undoCount:0,redoCount:0},
};

test('versioned session connection preserves unsigned 64-bit identities as decimal strings',()=>{
  const parsed=parseSessionConnection(valid);
  assert.deepEqual(parsed,valid);
  assert.equal(nextDecimalId('9007199254740993'),'9007199254740994');
  assert.equal(nextDecimalId('18446744073709551614'),'18446744073709551615');
  assert.throws(()=>nextDecimalId('18446744073709551615'),/exhausted/i);
  for(const changed of [
    {...valid,version:2},
    {...valid,instanceId:9007199254740993},
    {...valid,attachmentId:'01'},
    {...valid,revision:'-1'},
    {...valid,nextRequestId:'0'},
    {...valid,limits:{...valid.limits,outcomes:129}},
    {...valid,history:{undoCount:101,redoCount:0}},
  ])assert.throws(()=>parseSessionConnection(changed),/contract|identity|decimal|limit|history|version/i);
});

test('native projection retains exact map and zone identities independently of its editor rule id',()=>{
  const patch=patchFromSession(session());
  const kick=patch.rules.find(rule=>rule.id==='kit.kick');
  assert.equal(kick.nativeMapId,'kit');
  assert.equal(kick.nativeZoneId,'kick');
});


test('optional native active request preserves its u64 identity and rejects malformed or unconsumed IDs',()=>{
  const pending={...valid,nextRequestId:'9007199254740994',activeRequestId:'9007199254740993'};
  assert.deepEqual(parseSessionConnection(pending),pending);
  for(const activeRequestId of [null,undefined,0,9007199254740993,'0','01','-1','18446744073709551616','9007199254740994','9007199254740995']){
    assert.throws(()=>parseSessionConnection({...pending,activeRequestId}),/active|request|identity|decimal/i);
  }
});
