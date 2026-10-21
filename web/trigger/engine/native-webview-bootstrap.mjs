import {createWebViewTransport} from './native-webview-transport.mjs';

const installations=new WeakMap();
const fail=code=>{throw Object.assign(new Error(code),{code});};

/** Install once per document; disposal is terminal for this native attachment. */
export function installNativeWebViewTransport(scope=globalThis.window){
  const prior=scope&&installations.get(scope);
  if(!scope||!('__DANDRUM_NATIVE__' in scope)){
    if(prior)fail('nativeBootstrapConflict');
    return null;
  }
  const config=scope.__DANDRUM_NATIVE__;
  if(!config||typeof config!=='object'||Array.isArray(config)||config.protocolVersion!==1)fail('invalidNativeBootstrap');
  const {instanceId,attachmentId,nextRequestId='1'}=config;
  const ipc=scope.ipc;
  if(typeof ipc?.postMessage!=='function')fail('missingNativeIpc');
  if(prior){
    if(scope.__DANDRUM__!==prior.descriptor||instanceId!==prior.instanceId||attachmentId!==prior.attachmentId
      ||nextRequestId!==prior.nextRequestId||ipc!==prior.ipc||ipc.postMessage!==prior.postMessage)fail('nativeBootstrapConflict');
    if(prior.closed)fail('closed');
    return prior.handle;
  }
  if('__DANDRUM__' in scope)fail('nativeBootstrapConflict');
  const transport=createWebViewTransport({instanceId,attachmentId,nextRequestId,postMessage:ipc.postMessage.bind(ipc)});
  const descriptor={...transport.descriptor,receiveNativeMessage:transport.receive};
  const installation={descriptor,instanceId,attachmentId,nextRequestId,ipc,postMessage:ipc.postMessage,closed:false,handle:{dispose(){
    installation.closed=true;
    return transport.dispose();
  }}};
  scope.__DANDRUM__=descriptor;
  installations.set(scope,installation);
  return installation.handle;
}
