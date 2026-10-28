const events=new Set(['parameterStateChanged','instrumentSessionChanged']);
const encoder=new TextEncoder();
const U64_MAX=18446744073709551615n;
function decimal(value){
  return typeof value==='string'&&/^[1-9][0-9]{0,19}$/.test(value)&&BigInt(value)<=U64_MAX;
}
function failure(code){return Object.assign(new Error(code),{code});}
const patchMethods=['prepareInstrumentSessionPatch','getSamplerSession'];
function supportedPatchProfile(value){
  return value?.wireVersion===2&&Array.isArray(value.methods)&&value.methods.length===patchMethods.length
    &&value.methods.every((method,index)=>method===patchMethods[index])
    &&value.limits?.documentBytes===65536&&value.limits.requestBytes===524288
    &&value.limits.replyBytes===1048576&&value.limits.replayDocumentBytes===262144;
}

/**
 * Browser-side correlation for an admitted attachment with explicit v2 negotiation.
 * nextRequestId seeds this transport cursor, not the session's structural cursor.
 * postMessage is synchronous; only real native replies settle successful calls.
 * receive returns 'reply', 'event', or 'ignored'; dispose returns true only once.
 * Local failures are Errors with a code; native rejection preserves its error object.
 */
export function createWebViewTransport(options){
  if(!options||typeof options!=='object'||Array.isArray(options))throw failure('invalidConfiguration');
  const {instanceId,attachmentId,postMessage,nextRequestId='1'}=options;
  if(!decimal(instanceId)||!decimal(attachmentId)||!decimal(nextRequestId)||typeof postMessage!=='function')throw failure('invalidConfiguration');
  let next=BigInt(nextRequestId),lastEvent=0n,closed=false,patchProtocol=false;
  const pending=new Map(),listeners=new Map();
  const backend={
    getNativeFunction:method=>(...args)=>new Promise((resolve,reject)=>{
      if(closed)throw failure('closed');
      if(typeof method!=='string'||!method)throw failure('invalidArguments');
      let wireArgs;
      try{
        wireArgs=JSON.parse(JSON.stringify(args,(_key,value)=>{
          if(['undefined','function','symbol','bigint'].includes(typeof value)
            ||(typeof value==='number'&&!Number.isFinite(value)))throw failure('invalidArguments');
          return value;
        }));
      }catch{throw failure('invalidArguments');}
      // Serialization can run caller code (getters/toJSON). Admit only afterward.
      if(closed)throw failure('closed');
      if(next>U64_MAX)throw failure('identityExhausted');
      if(pending.size>=128)throw failure('pendingFull');
      const requestId=String(next);
      const version=patchProtocol&&patchMethods.includes(method)?2:1;
      if(version===2&&method==='prepareInstrumentSessionPatch'&&typeof wireArgs[4]==='string'
        &&encoder.encode(wireArgs[4]).length>65536)throw failure('oversized');
      const message=JSON.stringify({version,attachmentId,requestId,method,args:wireArgs});
      if(encoder.encode(message).length>(version===2?524288:16384))throw failure('oversized');
      next++;
      pending.set(requestId,{resolve,reject,version,negotiation:method==='negotiateEditorProtocol'});
      try{postMessage(message);}
      catch(error){
        if(pending.delete(requestId))reject(error);
      }
    }),
    addEventListener(event,listener){
      if(closed)throw failure('closed');
      if(!events.has(event)||typeof listener!=='function')throw failure('invalidArguments');
      if(listeners.size>=128)throw failure('listenersFull');
      const token=Symbol(event);
      listeners.set(token,{event,listener});
      return token;
    },
    removeEventListener:token=>listeners.delete(token),
  };
  // Returns 'reply', 'event', or 'ignored'. Listener exceptions propagate after consuming the sequence.
  function receive(message){
    if(closed)return 'ignored';
    if(typeof message!=='string')return 'ignored';
    const bytes=encoder.encode(message).length;
    if(bytes>(patchProtocol?1048576:65536))return 'ignored';
    let envelope;
    try{envelope=JSON.parse(message);}catch{return 'ignored';}
    if(!envelope||typeof envelope!=='object'||Array.isArray(envelope)
      ||![1,2].includes(envelope.version)||envelope.instanceId!==instanceId||envelope.attachmentId!==attachmentId
      ||(envelope.version===1&&bytes>65536))return 'ignored';
    if(Object.hasOwn(envelope,'event')){
      if(envelope.version!==1||!events.has(envelope.event)||!decimal(envelope.sequence)||!Object.hasOwn(envelope,'payload')
        ||['requestId','status','result','error'].some(key=>Object.hasOwn(envelope,key)))return 'ignored';
      const sequence=BigInt(envelope.sequence);
      if(sequence<=lastEvent)return 'ignored';
      lastEvent=sequence;
      // Snapshot limits callback-driven registration to the next event. A nested
      // newer event supersedes remaining older delivery; sequence gaps are valid.
      for(const [token,{event,listener}] of [...listeners]){
        if(lastEvent!==sequence)break;
        if(event===envelope.event&&listeners.has(token))listener(envelope.payload);
      }
      return 'event';
    }
    if(!decimal(envelope.requestId)||Object.hasOwn(envelope,'sequence')||Object.hasOwn(envelope,'payload'))return 'ignored';
    if(envelope.status==='accepted'){
      if(!Object.hasOwn(envelope,'result')||Object.hasOwn(envelope,'error'))return 'ignored';
    }else if(envelope.status==='rejected'){
      if(Object.hasOwn(envelope,'result')||!envelope.error||Array.isArray(envelope.error)
        ||typeof envelope.error.code!=='string'||!envelope.error.code)return 'ignored';
    }else return 'ignored';
    const request=pending.get(envelope.requestId);
    if(!request||envelope.version!==request.version)return 'ignored';
    pending.delete(envelope.requestId);
    if(envelope.status==='accepted'){
      if(request.negotiation){
        if(!supportedPatchProfile(envelope.result)){
          request.reject(failure('invalidProtocolProfile'));return 'reply';
        }
        patchProtocol=true;
      }
      request.resolve(envelope.result);
    }else request.reject(envelope.error);
    return 'reply';
  }
  const descriptor={protocolVersion:1,backendId:'juce-free',capabilities:{metering:false,sourceWaveforms:false,windowTitle:false,editorResize:false},backend};
  function dispose(){
    if(closed)return false;
    closed=true;
    listeners.clear();
    for(const request of pending.values())request.reject(failure('closed'));
    pending.clear();
    return true;
  }
  return {descriptor,receive,dispose};
}
