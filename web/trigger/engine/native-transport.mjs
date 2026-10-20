/** Select the native host contract; absence means the explicitly silent preview. */
export function selectNativeTransport(scope=globalThis.window){
  if(scope?.__DANDRUM__!==undefined){
    const transport=scope.__DANDRUM__;
    if(transport?.protocolVersion!==1)throw new Error('Unsupported native transport protocol version');
    if(typeof transport.backendId!=='string'||!transport.backendId
      ||['getNativeFunction','addEventListener','removeEventListener'].some(name=>typeof transport.backend?.[name]!=='function'))throw new Error('Invalid native transport contract');
    const capabilities={metering:true,sourceWaveforms:true,windowTitle:true,editorResize:true};
    if(Object.hasOwn(transport,'capabilities')){
      const profile=transport.capabilities;
      if(!profile||typeof profile!=='object'||Array.isArray(profile))throw new Error('Invalid native transport capabilities');
      for(const name of Object.keys(capabilities))if(Object.hasOwn(profile,name)){
        if(typeof profile[name]!=='boolean')throw new Error('Invalid native transport capabilities');
        capabilities[name]=profile[name];
      }
    }
    return {...transport,capabilities:Object.freeze(capabilities)};
  }
  const backend=scope?.__JUCE__?.backend;
  return typeof backend?.getNativeFunction==='function'?{protocolVersion:1,backendId:'juce',backend}:null;
}
