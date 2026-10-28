const MAX_U64=18446744073709551615n;
const invalid=()=>{throw new Error('Invalid native sample selection');};
const integer=(value,low,high)=>Number.isSafeInteger(value)&&value>=low&&value<=high;
const identity=value=>typeof value==='string'&&value.length<=20&&/^[1-9][0-9]*$/.test(value)&&BigInt(value)<=MAX_U64;
const text=value=>value instanceof Error?value.message:typeof value?.code==='string'?value.code:String(value);

function selectedFiles(files,selectionId,instanceId){
  if(!Array.isArray(files)||files.length<1||files.length>16)invalid();
  const owners=new Map();let bytes=0;
  return files.map(file=>{
    if(!file||typeof file!=='object'||typeof file.name!=='string'||!file.name.length||new TextEncoder().encode(file.name).length>256
      ||typeof file.contentSha256!=='string'||!/^[a-f0-9]{64}$/.test(file.contentSha256)
      ||file.resourceId!==`selected/${file.contentSha256}.wav`
      ||!integer(file.encodedBytes,44,67108864)||!integer(file.sampleRateHz,1,4294967295)
      ||!integer(file.channelCount,1,2)||!integer(file.frameCount,1,33554432)
      ||44+file.frameCount*file.channelCount*2>file.encodedBytes)invalid();
    const signature=[file.encodedBytes,file.sampleRateHz,file.channelCount,file.frameCount].join('/');
    if(owners.has(file.resourceId)){if(owners.get(file.resourceId)!==signature)invalid();}
    else{owners.set(file.resourceId,signature);bytes+=file.encodedBytes;if(bytes>268435456)invalid();}
    return {instanceId,name:file.name,uri:file.resourceId,resourceId:file.resourceId,selectionId,
      contentSha256:file.contentSha256,encodedBytes:file.encodedBytes,sampleRate:file.sampleRateHz,
      channels:file.channelCount,frameCount:file.frameCount,duration:file.frameCount/file.sampleRateHz};
  });
}

/** Native outcomes own bytes; this disposable consumer holds bounded metadata only. */
export async function chooseNativeSamples(engine){
  const before=engine.connection;
  const live=()=>!engine.closed&&engine.connection.instanceId===before.instanceId&&engine.connection.attachmentId===before.attachmentId;
  const assertLive=()=>{if(engine.closed)throw new Error('Engine is closed');if(!live())throw new Error('Native selection belongs to a stale connection');};
  assertLive();
  const admission=await engine.invoke('chooseSamplerFiles');assertLive();
  if(!identity(admission?.selectionId)||admission.phase!=='choosing')invalid();
  const id=admission.selectionId;let terminal=false,result,failed=false,failure;
  try{
    // Choosing waits for user input. Each read is bounded; close cancels the wait.
    for(;;){
      assertLive();const status=await engine.invoke('getSamplerSelectionStatus',id);assertLive();
      if(status?.selectionId!==id)invalid();
      if(status.phase==='ready'){terminal=true;result=selectedFiles(status.files,id,before.instanceId);terminal=false;break;}
      if(status.phase==='cancelled'){terminal=true;result=[];break;}
      if(status.phase==='failed'){
        terminal=true;
        if(typeof status.error!=='string'||!status.error.length||new TextEncoder().encode(status.error).length>256)invalid();
        throw Object.assign(new Error(`Native sample selection failed: ${status.error}`),{code:status.error});
      }
      if(!['choosing','preparing'].includes(status.phase))invalid();
      await engine.wait();
    }
  }catch(error){failed=true;failure=error;}
  if(terminal&&live())try{
    const released=await engine.invoke('releaseSamplerSelection',id);assertLive();
    if(released?.selectionId!==id||released.phase!=='released')invalid();
  }catch(error){
    failure=failed?new Error(`${text(failure)}; selection release failed: ${text(error)}`,{cause:failure}):error;failed=true;
  }
  if(failed)throw failure;
  return result;
}


function batchOwner(engine,files){
  if(!Array.isArray(files)||files.length<1||files.length>16||!identity(files[0]?.selectionId))invalid();
  const before=engine.connection,id=files[0].selectionId;
  if(files.some(file=>file?.instanceId!==before.instanceId||file.selectionId!==id))invalid();
  return {instanceId:before.instanceId,attachmentId:before.attachmentId,selectionId:id};
}
const current=(engine,owner)=>!engine.closed&&engine.connection.instanceId===owner.instanceId&&engine.connection.attachmentId===owner.attachmentId;
function assertCurrent(engine,owner){if(engine.closed)throw new Error('Engine is closed');if(!current(engine,owner))throw new Error('Native sample selection belongs to a stale connection');}
async function releaseOutcome(engine,owner){
  if(!current(engine,owner))return;
  const released=await engine.invoke('releaseSamplerSelection',owner.selectionId);assertCurrent(engine,owner);
  if(released?.selectionId!==owner.selectionId||released.phase!=='released')invalid();
}
export async function releaseNativeFiles(engine,files){await releaseOutcome(engine,batchOwner(engine,files));}

/** Consume one import choice; accepted native state/history retain their own leases. */
export async function withNativeFiles(engine,files,action){
  const owner=batchOwner(engine,files);let result,failed=false,failure;
  try{
    assertCurrent(engine,owner);
    if(files.some(file=>file.uri!==file.resourceId))invalid();
    const checked=selectedFiles(files.map(file=>({name:file.name,resourceId:file.resourceId,contentSha256:file.contentSha256,
      encodedBytes:file.encodedBytes,sampleRateHz:file.sampleRate,channelCount:file.channels,frameCount:file.frameCount})),owner.selectionId,owner.instanceId);
    result=await action(checked,()=>assertCurrent(engine,owner));assertCurrent(engine,owner);
  }catch(error){failed=true;failure=error;}
  try{await releaseOutcome(engine,owner);}
  catch(error){
    if(!failed)return {...result,warning:`Sample imported; temporary selection cleanup failed: ${text(error)}`};
    failure=new Error(`${text(failure)}; selection release failed: ${text(error)}`,{cause:failure});
  }
  if(failed)throw failure;
  return result;
}
