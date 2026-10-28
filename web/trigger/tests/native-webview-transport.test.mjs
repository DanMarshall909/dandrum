import test from 'node:test';
import assert from 'node:assert/strict';
import {createWebViewTransport} from '../engine/native-webview-transport.mjs';
import {selectNativeTransport} from '../engine/native-transport.mjs';

function connection(options={}){
  const sent=[];
  const transport=createWebViewTransport({instanceId:'17',attachmentId:'23',postMessage:message=>sent.push(message),...options});
  return {...transport,sent,call:(method,...args)=>transport.descriptor.backend.getNativeFunction(method)(...args)};
}
function reply(requestId,result,overrides={}){
  return JSON.stringify({version:1,instanceId:'17',attachmentId:'23',requestId,status:'accepted',result,...overrides});
}

test('the selected backend publishes positional version-1 requests and settles actual correlated outcomes',async()=>{
  const broker=connection();
  const selected=selectNativeTransport({__DANDRUM__:broker.descriptor});
  assert.equal(selected.backendId,'juce-free');
  assert.deepEqual(selected.capabilities,{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false});
  const first=selected.backend.getNativeFunction('setParameter')('drums.level',.37,7);
  const second=broker.call('getSamplerSession');
  assert.deepEqual(broker.sent.map(JSON.parse),[
    {version:1,attachmentId:'23',requestId:'1',method:'setParameter',args:['drums.level',.37,7]},
    {version:1,attachmentId:'23',requestId:'2',method:'getSamplerSession',args:[]},
  ]);
  assert.equal(broker.receive(reply('2',{generation:19,parameters:[]})),'reply');
  assert.deepEqual(await second,{generation:19,parameters:[]});
  const rejected=assert.rejects(first,reason=>reason.code==='capabilityUnavailable');
  assert.equal(broker.receive(reply('1',undefined,{status:'rejected',error:{code:'capabilityUnavailable'}})),'reply');
  await rejected;
});

test('the backend gives each listener a removable registration token',()=>{
  const {descriptor:{backend}}=connection();
  const one=backend.addEventListener('parameterStateChanged',()=>{});
  const two=backend.addEventListener('parameterStateChanged',()=>{});
  assert.notEqual(one,two);
  assert.equal(backend.removeEventListener(one),true);
  assert.equal(backend.removeEventListener(one),false);
  assert.equal(backend.removeEventListener(two),true);
});

test('connection identities and request cursors must be canonical nonzero u64 decimal strings',()=>{
  const invalid=[undefined,null,0,1,1n,'','0','01','+1','1.0','1e2',' 1','1 ','-1','١','18446744073709551616','9'.repeat(100)];
  for(const name of ['instanceId','attachmentId','nextRequestId']){
    for(const value of invalid){
      if(name==='nextRequestId'&&value===undefined)continue;
      assert.throws(()=>connection({[name]:value}),error=>error.code==='invalidConfiguration',`${name}: ${String(value)}`);
    }
  }
  for(const postMessage of [undefined,null,{},'send'])assert.throws(()=>connection({postMessage}),error=>error.code==='invalidConfiguration');
});

test('transport IDs retain precision above 2^53 and reject exhaustion without reusing the final u64',async()=>{
  for(const [seed,expected] of [['9007199254740993',['9007199254740993','9007199254740994']],['18446744073709551615',['18446744073709551615']]]){
    const broker=connection({instanceId:'18446744073709551615',nextRequestId:seed});
    for(const id of expected){
      const value=broker.call('getParameterState');
      assert.equal(JSON.parse(broker.sent.at(-1)).requestId,id);
      broker.receive(reply(id,{generation:33},{instanceId:'18446744073709551615'}));
      assert.deepEqual(await value,{generation:33});
    }
    if(seed==='18446744073709551615'){
      await assert.rejects(broker.call('getParameterState'),error=>error.code==='identityExhausted');
      assert.equal(broker.sent.length,1);
    }
  }
});

test('outgoing JSON is limited to 16 KiB of UTF-8 without retaining failed requests',async()=>{
  const broker=connection();
  // Compute the byte boundary independently from the literal wire envelope.
  const empty='{"version":1,"attachmentId":"23","requestId":"1","method":"unknown","args":[""]}';
  const fill='é'.repeat(Math.floor((16384-Buffer.byteLength(empty))/2));
  const exact=fill+'x'.repeat(16384-Buffer.byteLength(empty)-Buffer.byteLength(fill));
  const accepted=broker.call('unknown',exact);
  assert.equal(Buffer.byteLength(broker.sent[0]),16384);
  broker.receive(reply('1',null));assert.equal(await accepted,null);
  for(let index=0;index<129;index++){
    const rejected=assert.rejects(broker.call('unknown',exact+'é'),error=>error.code==='oversized');
    assert.equal(broker.sent.length,1);await rejected;
  }
  assert.equal(broker.sent.length,1);
  const next=broker.call('getParameterState');
  assert.equal(JSON.parse(broker.sent[1]).requestId,'2');
  broker.receive(reply('2','actual'));assert.equal(await next,'actual');
});

test('malformed method and non-JSON arguments reject before posting or retaining a request',async()=>{
  const broker=connection(),cycle={};cycle.self=cycle;
  for(const method of [null,undefined,12,{},'']){
    const rejected=assert.rejects(broker.call(method),error=>error.code==='invalidArguments');
    assert.equal(broker.sent.length,0);await rejected;
  }
  for(const arg of [undefined,()=>{},Symbol('value'),1n,NaN,Infinity,-Infinity,cycle,{nested:undefined},[undefined],{toJSON(){throw new Error('cannot serialize');}}]){
    await assert.rejects(broker.call('setParameter',arg),error=>error.code==='invalidArguments');
  }
  assert.deepEqual(broker.sent,[]);
  const pending=broker.call('setParameter','drums.level',.5,11);
  assert.equal(JSON.parse(broker.sent[0]).requestId,'1');
  broker.receive(reply('1',{status:'accepted'}));assert.deepEqual(await pending,{status:'accepted'});
});

test('at most 128 unresolved requests are retained and settlement frees capacity',async()=>{
  const broker=connection(),calls=Array.from({length:128},()=>broker.call('noteHeartbeat'));
  const full=broker.call('noteHeartbeat');
  assert.equal(broker.sent.length,128);
  await assert.rejects(full,error=>error.code==='pendingFull');
  broker.receive(reply('64',64));assert.equal(await calls[63],64);
  const replacement=broker.call('noteHeartbeat');
  assert.equal(JSON.parse(broker.sent.at(-1)).requestId,'129');
  for(let index=0;index<128;index++)if(index!==63)broker.receive(reply(String(index+1),index+1));
  broker.receive(reply('129',129));
  assert.deepEqual(await Promise.all([...calls,replacement]),Array.from({length:129},(_,index)=>index+1));
});

test('synchronous post failures reject their actual reason and release capacity without reusing IDs',async()=>{
  let shouldThrow=true;
  const failure=Object.assign(new Error('host rejected the post'),{code:'hostUnavailable'});
  const sent=[];
  const broker=connection({postMessage:message=>{sent.push(JSON.parse(message));if(shouldThrow)throw failure;}});
  for(let index=0;index<129;index++)await assert.rejects(broker.call('noteHeartbeat'),reason=>reason===failure);
  shouldThrow=false;
  const recovered=broker.call('noteHeartbeat');
  assert.equal(sent.length,130);assert.equal(sent.at(-1).requestId,'130');
  broker.receive(reply('130',{alive:true}));assert.deepEqual(await recovered,{alive:true});
});

test('a synchronous real reply can settle before postMessage returns or throws',async()=>{
  let broker;
  broker=connection({postMessage:message=>{
    const {requestId}=JSON.parse(message);
    assert.equal(broker.receive(reply(requestId,{generation:47})),'reply');
    throw new Error('after native reply');
  }});
  for(let index=0;index<129;index++)assert.deepEqual(await broker.call('getParameterState'),{generation:47});
});

test('malformed, foreign, unknown and retired replies leave unrelated promises pending',async()=>{
  const broker=connection(),outcomes=[];
  const one=broker.call('getSamplerSession').then(value=>outcomes.push(['one',value]));
  const two=broker.call('getParameterState').then(value=>outcomes.push(['two',value]));
  const malformed=[undefined,17,{},'', '{','null','[]','true',JSON.stringify({version:1}),
    reply('1','wrong',{version:2}),reply('1','wrong',{version:'1'}),reply('1','wrong',{instanceId:'18'}),
    reply('1','wrong',{attachmentId:'24'}),reply('1','wrong',{instanceId:17}),reply('1','wrong',{attachmentId:23}),
    ...['0','01','-1','1e0','18446744073709551616',1,null].map(requestId=>reply(requestId,'wrong')),
    reply('99','unknown'),reply('1','wrong',{status:'unexpected'}),reply('1',undefined),
    ...[undefined,null,[],{}, {code:7},{code:''}].map(error=>reply('1',undefined,{status:'rejected',error})),
    reply('1','wrong',{error:{code:'malformed'}}),reply('1','wrong',{status:'rejected',error:{code:'malformed'}}),
    reply('1','wrong',{event:'parameterStateChanged',sequence:'1',payload:{}}),
  ];
  for(const message of malformed)assert.equal(broker.receive(message),'ignored');
  await Promise.resolve();assert.deepEqual(outcomes,[]);
  assert.equal(broker.receive(reply('2',{actual:2})),'reply');await two;
  assert.equal(broker.receive(reply('2',{wrong:2})),'ignored');
  await Promise.resolve();assert.deepEqual(outcomes,[['two',{actual:2}]]);
  assert.equal(broker.receive(reply('1',{actual:1})),'reply');await one;
  assert.deepEqual(outcomes,[['two',{actual:2}],['one',{actual:1}]]);
});

test('received JSON admits exactly 64 KiB of UTF-8 and ignores larger messages',async()=>{
  const broker=connection(),pending=broker.call('getSamplerSession');
  const empty='{"version":1,"instanceId":"17","attachmentId":"23","requestId":"1","status":"accepted","result":""}';
  const fill='😀'.repeat(Math.floor((65536-Buffer.byteLength(empty))/4));
  const exact=fill+'x'.repeat(65536-Buffer.byteLength(empty)-Buffer.byteLength(fill));
  assert.equal(broker.receive(reply('1',exact+'é')),'ignored');
  assert.equal(Buffer.byteLength(reply('1',exact)),65536);
  assert.equal(broker.receive(reply('1',exact)),'reply');
  assert.equal(await pending,exact);
});

function event(sequence,payload,overrides={}){
  return JSON.stringify({version:1,instanceId:'17',attachmentId:'23',sequence,event:'parameterStateChanged',payload,...overrides});
}

test('supported native events preserve payloads and deliver increasing attachment-local sequences with gaps',()=>{
  const broker=connection(),backend=broker.descriptor.backend,seen=[];
  const token=backend.addEventListener('parameterStateChanged',payload=>seen.push(['parameter',payload]));
  backend.addEventListener('instrumentSessionChanged',payload=>seen.push(['session',payload]));
  const payload={generation:7,sequence:88,parameters:[{id:'drums.level',value:.73}]};
  assert.equal(broker.receive(event('9007199254740993',payload)),'event');
  for(const message of [event('9007199254740993',{}),event('1',{}),event('9007199254740994',{}, {instanceId:'18'}),
    event('9007199254740994',{}, {attachmentId:'24'}),event('9007199254740994',{}, {version:2}),
    ...['0','01',2,null,'18446744073709551616'].map(sequence=>event(sequence,{})),
    event('9007199254740994',{}, {event:'meterStateChanged'}),event('9007199254740994',undefined),
    event('9007199254740994',{}, {requestId:'1'}),event('9007199254740994',{}, {status:'accepted'}),
    event('9007199254740994','é'.repeat(32768)),
  ])assert.equal(broker.receive(message),'ignored');
  assert.deepEqual(seen,[['parameter',payload]]);
  assert.equal(broker.receive(event('9007199254740995',{revision:'9'},{event:'instrumentSessionChanged'})),'event');
  assert.equal(backend.removeEventListener(token),true);
  assert.equal(broker.receive(event('9007199254740996',{})),'event');
  assert.deepEqual(seen,[['parameter',payload],['session',{revision:'9'}]]);
});

test('listener admission rejects invalid registrations and shares one 128-entry bound across event names',()=>{
  const broker=connection(),backend=broker.descriptor.backend,seen=[];
  for(const name of [undefined,'','unknown',{},1])assert.throws(()=>backend.addEventListener(name,()=>{}),error=>error.code==='invalidArguments');
  for(const listener of [undefined,null,1,{}])assert.throws(()=>backend.addEventListener('parameterStateChanged',listener),error=>error.code==='invalidArguments');
  const tokens=Array.from({length:128},(_,index)=>backend.addEventListener(index%2?'instrumentSessionChanged':'parameterStateChanged',value=>seen.push([index,value])));
  assert.throws(()=>backend.addEventListener('instrumentSessionChanged',()=>{}),error=>error.code==='listenersFull');
  assert.equal(backend.removeEventListener(tokens[0]),true);
  backend.addEventListener('parameterStateChanged',value=>seen.push([128,value]));
  broker.receive(event('1',{actual:37}));
  assert.deepEqual(seen,Array.from({length:64},(_,index)=>[(index+1)*2,{actual:37}]));
});

test('reentrant event delivery never sends an older sequence after a newer one to remaining listeners',()=>{
  const broker=connection(),backend=broker.descriptor.backend,seen=[];
  backend.addEventListener('parameterStateChanged',value=>{
    seen.push(['first',value]);
    if(value===1)broker.receive(event('2',2));
  });
  backend.addEventListener('parameterStateChanged',value=>seen.push(['second',value]));
  assert.equal(broker.receive(event('1',1)),'event');
  assert.deepEqual(seen,[['first',1],['first',2],['second',2]]);
});

test('removing and adding listeners during delivery affects later events without unbounded iteration',()=>{
  const broker=connection(),backend=broker.descriptor.backend,seen=[];
  let removed;
  backend.addEventListener('parameterStateChanged',value=>{
    seen.push(['first',value]);backend.removeEventListener(removed);
    backend.addEventListener('parameterStateChanged',value=>seen.push(['new',value]));
  });
  removed=backend.addEventListener('parameterStateChanged',value=>seen.push(['removed',value]));
  broker.receive(event('1',1));assert.deepEqual(seen,[['first',1]]);
  broker.receive(event('2',2));assert.deepEqual(seen,[['first',1],['first',2],['new',2]]);
});

test('a listener exception is visible and does not make its delivered sequence replayable',()=>{
  const broker=connection(),backend=broker.descriptor.backend,seen=[];
  const failure=new Error('consumer failed');
  const token=backend.addEventListener('parameterStateChanged',()=>{throw failure;});
  backend.addEventListener('parameterStateChanged',value=>seen.push(value));
  assert.throws(()=>broker.receive(event('1',1)),error=>error===failure);
  assert.equal(broker.receive(event('1',1)),'ignored');
  backend.removeEventListener(token);
  assert.equal(broker.receive(event('2',2)),'event');assert.deepEqual(seen,[2]);
});

test('disposal rejects every pending call once, clears listeners and makes later work inert',async()=>{
  const broker=connection(),backend=broker.descriptor.backend,seen=[];
  const cached=backend.getNativeFunction('noteHeartbeat');
  const token=backend.addEventListener('parameterStateChanged',value=>seen.push(value));
  const settlements=Array.from({length:128},()=>broker.call('getSamplerSession').then(
    ()=>assert.fail('disposed call resolved'),error=>{assert.equal(error.code,'closed');seen.push('closed');},
  ));
  assert.equal(broker.dispose(),true);assert.equal(broker.dispose(),false);
  await Promise.all(settlements);assert.equal(seen.length,128);
  assert.equal(backend.removeEventListener(token),false);
  assert.throws(()=>backend.addEventListener('parameterStateChanged',()=>{}),error=>error.code==='closed');
  await assert.rejects(cached(),error=>error.code==='closed');
  await assert.rejects(broker.call('noteHeartbeat'),error=>error.code==='closed');
  assert.equal(broker.sent.length,128);
  assert.equal(broker.receive(reply('1','late')),'ignored');
  assert.equal(broker.receive(event('1','late')),'ignored');
  assert.equal(broker.receive('{'),'ignored');
  assert.deepEqual(seen,Array(128).fill('closed'));
});

test('a disposed connection does not affect a fresh independent attachment or its event sequence',async()=>{
  const old=connection();old.receive(event('9007199254740993',{}));old.dispose();
  const fresh=connection({attachmentId:'24'}),seen=[];
  fresh.descriptor.backend.addEventListener('parameterStateChanged',value=>seen.push(value));
  const pending=fresh.call('getSamplerSession');
  assert.equal(fresh.receive(reply('1','old')),'ignored');
  assert.equal(fresh.receive(event('1','fresh',{attachmentId:'24'})),'event');
  assert.equal(fresh.receive(reply('1','fresh',{attachmentId:'24'})),'reply');
  assert.equal(await pending,'fresh');assert.deepEqual(seen,['fresh']);
  assert.equal(fresh.dispose(),true);
});

test('disposal inside a callback stops remaining delivery and can reject an in-flight synchronous post',async()=>{
  const broker=connection(),backend=broker.descriptor.backend,seen=[];
  backend.addEventListener('parameterStateChanged',value=>{seen.push(value);broker.dispose();});
  backend.addEventListener('parameterStateChanged',()=>assert.fail('callback after disposal'));
  assert.equal(broker.receive(event('1',1)),'event');assert.deepEqual(seen,[1]);
  let duringPost;
  duringPost=connection({postMessage:()=>{duringPost.dispose();throw new Error('post threw after disposal');}});
  await assert.rejects(duringPost.call('getSamplerSession'),error=>error.code==='closed');
});

test('argument serialization callbacks cannot reuse a transport ID or publish after disposal',async()=>{
  const broker=connection();let nested;
  const outer=broker.call('unknown',{toJSON(){nested=broker.call('noteHeartbeat');return 'serialized';}});
  assert.deepEqual(broker.sent.map(JSON.parse),[
    {version:1,attachmentId:'23',requestId:'1',method:'noteHeartbeat',args:[]},
    {version:1,attachmentId:'23',requestId:'2',method:'unknown',args:['serialized']},
  ]);
  broker.receive(reply('1','inner'));broker.receive(reply('2','outer'));
  assert.equal(await nested,'inner');assert.equal(await outer,'outer');
  const closing=connection();
  const rejected=assert.rejects(closing.call('unknown',{toJSON(){closing.dispose();return 'serialized';}}),error=>error.code==='closed');
  assert.deepEqual(closing.sent,[]);await rejected;
});

test('serialization callbacks cannot exceed pending capacity or the final transport ID',async()=>{
  const broker=connection();let nested;
  const rejected=assert.rejects(broker.call('unknown',{toJSON(){nested=Array.from({length:128},()=>broker.call('noteHeartbeat'));return 0;}}),error=>error.code==='pendingFull');
  assert.equal(broker.sent.length,128);await rejected;
  for(let index=0;index<128;index++)broker.receive(reply(String(index+1),index));
  assert.deepEqual(await Promise.all(nested),Array.from({length:128},(_,index)=>index));
  const last=connection({nextRequestId:'18446744073709551615'});let final;
  const exhausted=assert.rejects(last.call('unknown',{toJSON(){final=last.call('noteHeartbeat');return 0;}}),error=>error.code==='identityExhausted');
  assert.equal(last.sent.length,1);await exhausted;
  last.receive(reply('18446744073709551615','last'));assert.equal(await final,'last');
});

test('missing or invalid connection options fail with the same explicit configuration error',()=>{
  for(const options of [undefined,null,1,'connection',[],{}])assert.throws(()=>createWebViewTransport(options),error=>error.code==='invalidConfiguration');
});


const patchProfile=()=>({wireVersion:2,methods:['prepareInstrumentSessionPatch','getSamplerSession'],limits:{documentBytes:65536,requestBytes:524288,replyBytes:1048576,replayDocumentBytes:262144}});

test('a correlated negotiated profile carries large patch work alongside outstanding legacy replies and events',async t=>{
  const broker=connection(),seen=[];
  const call=(...args)=>{const result=broker.call(...args);result.catch(()=>{});return result;};
  t.after(()=>broker.dispose());
  broker.descriptor.backend.addEventListener('instrumentSessionChanged',value=>seen.push(value));
  const legacy=call('getSamplerSession'),negotiation=call('negotiateEditorProtocol',2);
  assert.deepEqual(broker.sent.map(JSON.parse),[
    {version:1,attachmentId:'23',requestId:'1',method:'getSamplerSession',args:[]},
    {version:1,attachmentId:'23',requestId:'2',method:'negotiateEditorProtocol',args:[2]},
  ]);
  assert.equal(broker.receive(reply('2',patchProfile(),{version:2})),'ignored');
  assert.equal(broker.receive(reply('2',patchProfile())),'reply');
  assert.deepEqual(await negotiation,patchProfile());
  const yaml='é'.repeat(10000),patch=call('prepareInstrumentSessionPatch','17','23','91','0',yaml);
  const posted=JSON.parse(broker.sent.at(-1));
  assert.equal(posted.version,2);assert.equal(posted.method,'prepareInstrumentSessionPatch');
  assert.deepEqual({...posted,args:posted.args.slice(0,4)},{version:2,attachmentId:'23',requestId:'3',method:'prepareInstrumentSessionPatch',args:['17','23','91','0']});
  assert.equal(posted.args[4],yaml);
  const snapshot=call('getSamplerSession'),parameter=call('getParameterState');
  assert.deepEqual(broker.sent.slice(-2).map(JSON.parse).map(({version,requestId})=>[version,requestId]),[[2,'4'],[1,'5']]);
  assert.equal(broker.receive(reply('3','wrong legacy version')),'ignored');
  assert.equal(broker.receive(reply('4','wrong legacy version')),'ignored');
  assert.equal(broker.receive(reply('5','wrong new version',{version:2})),'ignored');
  assert.equal(broker.receive(event('1',{revision:'1'},{event:'instrumentSessionChanged'})),'event');
  assert.equal(broker.receive(reply('1','x'.repeat(65536))),'ignored','old reply keeps its original bound');
  assert.equal(broker.receive(reply('4','😀'.repeat(20000),{version:2})),'reply');
  assert.equal(await snapshot,'😀'.repeat(20000));
  assert.equal(broker.receive(reply('3',{status:'accepted',job_id:'91'},{version:2})),'reply');
  assert.deepEqual(await patch,{status:'accepted',job_id:'91'});
  assert.equal(broker.receive(reply('1','old snapshot')),'reply');assert.equal(await legacy,'old snapshot');
  assert.equal(broker.receive(reply('5',{generation:9})),'reply');assert.deepEqual(await parameter,{generation:9});
  assert.deepEqual(seen,[{revision:'1'}]);
  const denied=call('prepareInstrumentSessionPatch','17','23','91','0',yaml);
  const rejected=assert.rejects(denied,error=>error.code==='conflictingRequest'&&error.message==='Document identity changed');
  assert.equal(broker.receive(reply('6',undefined,{version:2,status:'rejected',error:{code:'conflictingRequest',message:'Document identity changed'}})),'reply');
  await rejected;
});


async function negotiate(broker){
  const pending=broker.call('negotiateEditorProtocol',2),id=JSON.parse(broker.sent.at(-1)).requestId;
  broker.receive(reply(id,patchProfile()));assert.deepEqual(await pending,patchProfile());
}

test('only an exact supported correlated negotiation enables the new profile on that attachment',async()=>{
  const bad=[null,[],{},
    {...patchProfile(),wireVersion:'2'}, {...patchProfile(),wireVersion:3},
    {...patchProfile(),methods:null}, {...patchProfile(),methods:['getSamplerSession']},
    {...patchProfile(),methods:[...patchProfile().methods,'setParameter']},
    {...patchProfile(),methods:[...patchProfile().methods].reverse()},
    {...patchProfile(),limits:null}, {...patchProfile(),limits:{...patchProfile().limits,documentBytes:65537}},
    {...patchProfile(),limits:{...patchProfile().limits,requestBytes:'524288'}},
    {...patchProfile(),limits:{...patchProfile().limits,replyBytes:2097152}},
    {...patchProfile(),limits:{...patchProfile().limits,replayDocumentBytes:0}},
  ];
  for(const result of bad){
    const broker=connection(),pending=broker.call('negotiateEditorProtocol',2);
    const rejected=assert.rejects(pending,error=>error.code==='invalidProtocolProfile');
    assert.equal(broker.receive(reply('1',result)),'reply');await rejected;
    const snapshot=broker.call('getSamplerSession');
    assert.equal(JSON.parse(broker.sent.at(-1)).version,1,'bad negotiation retains legacy framing');
    broker.receive(reply('2',null));await snapshot;broker.dispose();
  }
  const broker=connection(),unrelated=broker.call('getParameterState');
  broker.receive(reply('1',patchProfile()));await unrelated;
  const denied=broker.call('negotiateEditorProtocol',2),rejected=assert.rejects(denied,error=>error.code==='capabilityUnavailable');
  broker.receive(reply('2',undefined,{status:'rejected',error:{code:'capabilityUnavailable'}}));await rejected;
  const legacy=broker.call('getSamplerSession');assert.equal(JSON.parse(broker.sent.at(-1)).version,1);
  broker.receive(reply('3',null));await legacy;
  await negotiate(broker);await negotiate(broker);
  const fresh=connection({attachmentId:'24'}),snapshot=fresh.call('getSamplerSession');
  assert.equal(JSON.parse(fresh.sent.at(-1)).version,1,'negotiation never spreads to another view');
  fresh.receive(reply('1',null,{attachmentId:'24'}));await snapshot;fresh.dispose();broker.dispose();
});

test('negotiated framing enforces independent document, wire, reply and unchanged legacy budgets',async t=>{
  const broker=connection();t.after(()=>broker.dispose());await negotiate(broker);
  // A maximum UTF-8 document with worst-case JSON escaping still fits the
  // independent request cap. Decoded bytes, not JavaScript code units, count.
  for(const yaml of ['é'.repeat(32768),'\u0000'.repeat(65536)]){
    const pending=broker.call('prepareInstrumentSessionPatch','17','23','9','0',yaml);
    const frame=JSON.parse(broker.sent.at(-1));assert.equal(frame.version,2);
    assert.equal(Buffer.byteLength(frame.args[4]),65536);assert.ok(Buffer.byteLength(broker.sent.at(-1))<=524288);
    broker.receive(reply(frame.requestId,null,{version:2}));await pending;
    const count=broker.sent.length;
    await assert.rejects(broker.call('prepareInstrumentSessionPatch','17','23','9','0',yaml+'é'),error=>error.code==='oversized');
    assert.equal(broker.sent.length,count,'decoded document overflow is unposted');
  }
  await assert.rejects(broker.call('setParameter','x'.repeat(16384)),error=>error.code==='oversized');
  // Framing-only calibration: arbitrary arguments reach native validation;
  // this does not claim the native snapshot method accepts arguments.
  const nextId=String(BigInt(JSON.parse(broker.sent.at(-1)).requestId)+1n);
  const empty=JSON.stringify({version:2,attachmentId:'23',requestId:nextId,method:'getSamplerSession',args:['']});
  const exact='x'.repeat(524288-Buffer.byteLength(empty));
  await assert.rejects(broker.call('getSamplerSession',exact+'x'),error=>error.code==='oversized');
  const pending=broker.call('getSamplerSession',exact);assert.equal(Buffer.byteLength(broker.sent.at(-1)),524288);
  const emptyReply=reply(nextId,'',{version:2}),value='x'.repeat(1048576-Buffer.byteLength(emptyReply));
  assert.equal(Buffer.byteLength(reply(nextId,value,{version:2})),1048576);
  assert.equal(broker.receive(reply(nextId,value+'x',{version:2})),'ignored');
  assert.equal(broker.receive(event('1','x'.repeat(65536))),'ignored','events retain the legacy limit');
  assert.equal(broker.receive(event('1',null,{version:2})),'ignored','events retain version one');
  assert.equal(broker.receive(reply(nextId,value,{version:2})),'reply');assert.equal(await pending,value);
  const retiring=broker.call('getSamplerSession'),closed=assert.rejects(retiring,error=>error.code==='closed');
  broker.dispose();await closed;assert.equal(broker.receive(reply(String(BigInt(nextId)+1n),null,{version:2})),'ignored');
});
