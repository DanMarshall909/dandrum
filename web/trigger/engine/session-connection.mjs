const U64_MAX=18446744073709551615n;
const positive=/^[1-9][0-9]*$/;
const unsigned=/^(0|[1-9][0-9]*)$/;
const decimal=(value,name,{zero=false}={})=>{
  if(typeof value!=='string'||!(zero?unsigned:positive).test(value)||BigInt(value)>U64_MAX)throw new Error(`Invalid session ${name} decimal identity`);
  return value;
};
const bounded=(value,name,expected)=>{
  if(!Number.isSafeInteger(value)||value!==expected)throw new Error(`Unsupported session ${name} limit`);
  return value;
};

/** Validate the Rust-owned v1 connection contract without converting u64 values to Number. */
export function parseSessionConnection(value){
  if(!value||typeof value!=='object'||value.version!==1)throw new Error('Unsupported instrument session contract version');
  const limits=value.limits,history=value.history;
  if(!limits||!history)throw new Error('Incomplete instrument session contract');
  const parsed={
    version:1,
    instanceId:decimal(value.instanceId,'instance'),
    attachmentId:decimal(value.attachmentId,'attachment'),
    revision:decimal(value.revision,'revision',{zero:true}),
    nextRequestId:decimal(value.nextRequestId,'request'),
    limits:{
      attachments:bounded(limits.attachments,'attachment',16),
      history:bounded(limits.history,'history',100),
      outcomes:bounded(limits.outcomes,'outcome',128),
      activeRequests:bounded(limits.activeRequests,'active request',1),
    },
    history:{undoCount:history.undoCount,redoCount:history.redoCount},
  };
  if(!Number.isSafeInteger(history.undoCount)||!Number.isSafeInteger(history.redoCount)
    ||history.undoCount<0||history.redoCount<0||history.undoCount+history.redoCount>parsed.limits.history)throw new Error('Invalid instrument session history bounds');
  if(Object.hasOwn(value,'activeRequestId')){
    const active=decimal(value.activeRequestId,'active request');
    if(BigInt(active)>=BigInt(parsed.nextRequestId))throw new Error('Invalid session active request identity');
    parsed.activeRequestId=active;
  }
  return parsed;
}

export function nextDecimalId(value){
  const current=BigInt(decimal(value,'request'));
  if(current===U64_MAX)throw new Error('Instrument session request identity exhausted');
  return String(current+1n);
}
