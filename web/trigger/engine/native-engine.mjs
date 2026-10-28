import {emptyPatch} from './presets.mjs';
import {patchFromSession,compileNativePatch,compiledNativeZoneId} from './native-document.mjs';
import {NativeLive} from './native-live.mjs';
import {createParameterController} from '../../shared/parameter-controller.mjs';
import {structureOperations} from './domains/structure.mjs';
import {instrumentOperations} from './domains/instrument.mjs';
import {historyOperations} from './domains/history.mjs';
import {sliceOperations} from './domains/slices.mjs';
import {candidateOperations} from './domains/candidates.mjs';
import {routingOperations} from './domains/routing.mjs';
import {mapAssets} from './domains/import-mapping.mjs';
import {applyEntityDelta} from './entity-delta.mjs';
import {synchroniseModuleParameters} from './voice-defaults.mjs';
import {parse} from 'yaml';
import {parseSessionConnection,nextDecimalId} from './session-connection.mjs';
import {selectNativeTransport} from './native-transport.mjs';
import {chooseNativeSamples,releaseNativeFiles,withNativeFiles} from './native-selection.mjs';

const operations={...structureOperations,...instrumentOperations,...historyOperations,...sliceOperations,...candidateOperations,...routingOperations};
const unsupported=new Set(['collapseHistory','addModulator','setModulator','removeModulator','addRoute','updateRoute','removeRoute','setMacro','bindMacro','learnMidi','receiveMidi','addProcessor','moveProcessor','setProcessorParam','removeProcessor']);
// The JUCE-free document stores DSP state, not browser hierarchy or processing stacks.
const nativeEditFields={
  setVoicePolicy:[0,['limit','steal']],
  updateRule:[1,['noteLo','noteHi','velLo','velHi','root']],
  setRegion:[1,['start','end','root','reverse','gain','pan','fades','loop','playback']],
  setCandidate:[2,['weight','gain','pan','tune','nativeChokeGroup']],
  setSelector:[1,['policy']],
};
const nativeEditMethods=new Set(['renameNode','setRules','removeRule','removeCandidate','reorderCandidates',...Object.keys(nativeEditFields)]);
function errorMessage(error){
  return error instanceof Error?error.message
    :typeof error?.message==='string'?error.message
    :typeof error?.code==='string'?error.code
    :String(error);
}
/** Native preparation is the only publication boundary. Domain edits remain plain data. */
export class NativeEngine {
  constructor({backend,pollMs=30,heartbeatMs=1000}={}){
    const transport=backend===undefined?selectNativeTransport():{backendId:'juce',backend};
    backend=transport?.backend;
    if(typeof backend?.getNativeFunction!=='function')throw new Error('Native sampler backend unavailable');
    this.backendId=transport.backendId;
    if(this.backendId==='juce-free')this.redoSession=()=>this.connection?.history.redoCount?this.publishSessionCommand('redoInstrumentSession',[]):Promise.resolve();
    this.backend=backend;this.pollMs=pollMs;this.heartbeatMs=heartbeatMs;this.patch=emptyPatch();this.patch.macros=[];
    this.listeners=new Map();this.closed=false;this.sequence=0;this.pending=null;this.waits=new Set();this.eventTokens=[];
    this.pendingGeneration=0;this.preparing=false;this.reconciliation=null;
    this.native=true;this.capabilities=Object.freeze({analysis:false,zeroCrossings:false,fx:false,modulation:false,wholeSourceAudition:false,metering:true,sourceWaveforms:true,windowTitle:true,editorResize:true,...transport.capabilities});
  }
  async initialize(){
    if(this.backendId==='juce-free')await this.invoke('negotiateEditorProtocol',2);
    this.session=await this.invoke('getSamplerSession');this.connection=parseSessionConnection(this.session.connection);this.patch=patchFromSession(this.session);this.generation=this.session.document.generation;
    this.sourcePath=this.session.sourcePath;this.samplerSession=this.session;this.loadedPreset='native';
    await this.publishWindowTitle();
    this.live=new NativeLive(this);await this.startFeedback();
    this.parameters=createParameterController((name,...args)=>this.invoke(name,...args),state=>{
      if(this.closed)return;
      if(state.generation>this.generation){void this.requestGeneration(state.generation);return;}
      if(state.generation!==this.generation)return;
      for(const parameter of state.parameters){const current=this.patch.params[parameter.id];if(current&&current.value!==parameter.value){current.value=parameter.value;synchroniseModuleParameters(this.patch,parameter.id,parameter.value);this.emit('param',{id:parameter.id,value:parameter.value});}}
    },error=>{if(error)this.report(error);});
    this.parameters.accept({generation:this.generation,sequence:0,parameters:Object.values(this.patch.params).map(p=>({id:p.id,name:p.name,value:p.value}))});
    if(this.backend.addEventListener)this.eventTokens.push(this.backend.addEventListener('parameterStateChanged',state=>{
      if(state?.generation>this.generation)return this.requestGeneration(state.generation);
      this.parameters.accept(state);
    }));
    if(this.heartbeatMs>0){
      this.heartbeatTimer=setInterval(()=>void this.live.notes.keepAlive(),this.heartbeatMs);
      if(this.capabilities.metering)this.meterTimer=setInterval(()=>void this.pollTelemetry().catch(error=>this.report(error)),50);
    }
    return this;
  }
  async startFeedback(changed=false){
    const started=changed?await this.live.changed():await this.live.resetMeter();
    if(!started&&!await this.live.resetMeter())throw new Error('Native output meter subscription unavailable');
  }
  requestGeneration(generation){
    if(this.closed||!Number.isInteger(generation)||generation<=this.generation)return;
    this.pendingGeneration=Math.max(this.pendingGeneration,generation);
    if(this.preparing||this.reconciliation)return this.reconciliation;
    let completed=false;
    const pending=this.serial(async()=>{
      if(this.pendingGeneration<=this.generation)return;
      const requested=this.pendingGeneration;
      const session=await this.invoke('getSamplerSession');
      if(this.closed)return;
      if(session.document?.generation<requested)throw new Error('Native session generation is stale');
      const accepted=patchFromSession(session),connection=parseSessionConnection(session.connection);
      const replacement=connection.instanceId!==this.connection.instanceId||connection.attachmentId!==this.connection.attachmentId
        ||session.sourcePath!==this.sourcePath||JSON.stringify(parse(session.yaml))!==JSON.stringify(parse(this.session.yaml));
      this.session=session;this.generation=session.document.generation;this.sourcePath=session.sourcePath;
      this.connection=connection;
      if(this.pendingGeneration<=this.generation)this.pendingGeneration=0;
      if(this.backendId==='juce-free'||accepted.rules.length)this.samplerSession=session;
      if(replacement)this.patch=accepted;
      else this.patch.params=accepted.params;
      await this.startFeedback(true);
      this.parameters.accept({generation:this.generation,sequence:0,parameters:Object.values(this.patch.params).map(p=>({id:p.id,name:p.name,value:p.value}))});
      await this.publishWindowTitle();
      this.emit('patch',{phase:'ready',patch:this.getPatch(),replacement});
      completed=true;
    });
    this.reconciliation=pending;
    pending.catch(error=>this.report(error)).finally(()=>{this.reconciliation=null;if(completed&&this.pendingGeneration>this.generation)void this.requestGeneration(this.pendingGeneration);});
    return pending;
  }
  async invoke(name,...args){
    const value=await this.backend.getNativeFunction(name)(...args);
    if(typeof value==='string'&&!(name==='getSamplerPresetPath'&&value.startsWith('/')))throw new Error(value);
    if(value?.status&&value.status!=='accepted')throw new Error(`Native ${name} rejected: ${value.status}`);
    return value;
  }
  on(event,listener){if(!this.listeners.has(event))this.listeners.set(event,new Set());this.listeners.get(event).add(listener);return()=>this.listeners.get(event)?.delete(listener);}
  emit(event,value){if(!this.closed)for(const listener of this.listeners.get(event)??[])listener(value);}
  report(error){
    if(this.closed)return;
    const message=errorMessage(error);
    this.emit('patch',{phase:'failed',message});
  }
  async publishWindowTitle(){
    if(!this.capabilities.windowTitle)return;
    try{await this.invoke('setTriggerWindowTitle',this.patch.name);}
    catch(error){this.report(new Error(`Window title update failed: ${error instanceof Error?error.message:String(error)}`));}
  }
  getTelemetrySnapshot(){return structuredClone(this.live.telemetry);}
  getPatch(){return structuredClone(this.patch);}
  id(prefix){let id;const used=new Set(Object.values(this.patch).flatMap(value=>Array.isArray(value)?value.map(item=>item.id):[]));do{id=`${prefix}-${++this.sequence}`;}while(used.has(id));return id;}
  wait(){return new Promise((resolve,reject)=>{const entry={timer:setTimeout(()=>{this.waits.delete(entry);resolve();},this.pollMs),reject};this.waits.add(entry);});}
  async pollJob(name,id,ready='completed'){
    for(let attempt=0;attempt<2000;attempt++){
      if(this.closed)throw new Error('Engine is closed');
      const reply=await this.invoke(name,id);
      if(reply?.state===ready)return reply;
      if(['failed','stale','cancelled'].includes(reply?.state))throw new Error(reply.error||`Native preparation ${reply.state}`);
      if(reply?.state!=='running')throw new Error('Unknown native job state');await this.wait();
    }
    throw new Error('Native preparation timed out');
  }
  serial(action){
    const pending=(this.pending??Promise.resolve()).catch(()=>{}).then(()=>{if(this.closed)throw new Error('Engine is closed');return action();});
    this.pending=pending;pending.finally(()=>{if(this.pending===pending)this.pending=null;}).catch(()=>{});return pending;
  }
  prepared(edit,{reloadPath}={}){
    return this.serial(async()=>{
      this.preparing=true;
      const sessionOwned=this.backendId==='juce-free'&&!reloadPath,before=this.connection;
      this.emit('patch',{phase:'preparing'});
      try{
        const draft=this.getPatch(),result=edit(draft);
        if(draft.rules.some(rule=>rule.source)&&!Object.keys(draft.params).length)draft.params=patchFromSession(this.samplerSession).params;
        const text=reloadPath?null:compileNativePatch(draft,this.samplerSession,{retainPublicSurface:this.backendId==='juce-free'}),generation=this.generation;
        const request=before.nextRequestId;
        const admitted=await (sessionOwned
          ?this.invoke('prepareInstrumentSessionPatch',before.instanceId,before.attachmentId,request,before.revision,text)
          :this.invoke(reloadPath?'reloadInstrument':'prepareSampler',reloadPath??text,generation));
        if(admitted?.status!=='accepted'||(sessionOwned?admitted.job_id!==request:admitted.generation!==generation||!Number.isInteger(admitted.job_id)))throw new Error('Native preparation was not admitted');
        const completed=await this.pollJob(sessionOwned?'getInstrumentSessionJobStatus':'getUiJobStatus',admitted.job_id);
        if(this.closed)throw new Error('Engine is closed');
        const session=await this.invoke('getSamplerSession');
        if(this.closed)throw new Error('Engine is closed');
        if(session.document?.generation<=generation||(!sessionOwned&&completed.generation!==session.document.generation))throw new Error('Native preparation completed with stale generation');
        const accepted=patchFromSession(session),connection=parseSessionConnection(session.connection); // Validate real file/frame metadata before publication.
        if(sessionOwned&&(connection.instanceId!==before.instanceId||connection.attachmentId!==before.attachmentId||completed.request_id!==request||completed.revision!==connection.revision||connection.nextRequestId!==nextDecimalId(request)))throw new Error('Native document preparation returned incoherent identity or revision metadata');
        this.session=session;this.generation=session.document.generation;this.sourcePath=session.sourcePath;
        this.connection=connection;
        if(this.backendId==='juce-free'||accepted.rules.length)this.samplerSession=session;
        this.patch=sessionOwned||reloadPath?accepted:draft;
        this.patch.params=accepted.params;
        if(!reloadPath&&!sessionOwned)this.reconcilePreparedRuleIdentities(accepted);
        // Native geometry is authoritative; keep stable editor identities and undo operands.
        for(const asset of this.patch.assets){const decoded=accepted.assets.find(a=>a.id===asset.id||a.uri===asset.uri);if(decoded)Object.assign(asset,{sampleRate:decoded.sampleRate,channels:decoded.channels,duration:decoded.duration,frameCount:decoded.frameCount});}
        await this.startFeedback(true);
        this.parameters.accept({generation:this.generation,sequence:0,parameters:Object.values(this.patch.params).map(p=>({id:p.id,name:p.name,value:p.value}))});
        await this.publishWindowTitle();
        this.emit('patch',{phase:'ready',patch:this.getPatch()});return structuredClone(result);
      }catch(error){
        let failure=error;
        if(sessionOwned&&!this.closed)try{
          const session=await this.invoke('getSamplerSession');
          if(!this.closed){this.acceptSessionCommandSnapshot(session,before,true);await this.startFeedback(true);}
        }catch(refreshError){failure=new Error(`${errorMessage(error)}; session refresh failed: ${errorMessage(refreshError)}`,{cause:error});}
        this.report(failure);throw failure;
      }
      finally{this.preparing=false;if(this.pendingGeneration>this.generation)void this.requestGeneration(this.pendingGeneration);else this.pendingGeneration=0;}
    });
  }
  assertNativeEdit(name,args){
    if(this.backendId!=='juce-free')return;
    const reject=()=>{throw new Error(`${name} contains state unsupported by native document history`);};
    if(!nativeEditMethods.has(name))reject();
    const fields=nativeEditFields[name];
    if(fields&&Object.keys(args[fields[0]]??{}).some(key=>!fields[1].includes(key)))reject();
    if(name==='renameNode'&&!this.patch.nodes.some(node=>node.id===args[0]&&node.parent===null))reject();
    if(name==='setSelector'&&!['rr','weighted'].includes(args[1]?.policy))reject();
    if(name==='setRules')for(const rule of args[0]){
      const prior=this.patch.rules.find(item=>item.id===rule.id);
      if(!prior||[...new Set([...Object.keys(prior),...Object.keys(rule)])].some(key=>!nativeEditFields.updateRule[1].includes(key)&&JSON.stringify(prior[key])!==JSON.stringify(rule[key])))reject();
    }
  }
  edit(name,args){
    try{this.assertNativeEdit(name,args);}catch(error){return Promise.reject(error);}
    if(unsupported.has(name))return Promise.reject(new Error(`${name} is unsupported by the native sampler`));
    if(name==='setModuleParam')return Promise.reject(new Error('Voice module configuration is unsupported; use exposed native parameters'));
    if(['setHistory','addHistory'].includes(name)&&args.some(value=>value?.config?.snap||value?.snap))return Promise.reject(new Error('Zero crossing analysis is unsupported by the native sampler'));
    return this.prepared(draft=>operations[name].call(this,draft,...args));
  }
  ownsOperation(method,...args){
    if(this.backendId==='juce-free'&&!unsupported.has(method)&&method!=='setModuleParam'
      &&(Object.hasOwn(operations,method)||['importAssets','importSamples'].includes(method)))return true;
    return this.ownsKeyRangeOperation(method,...args);
  }
  ownsKeyRangeOperation(method,...args){
    if(method!=='updateRule')return false;
    const [id,delta]=args,rule=this.patch.rules.find(item=>item.id===id),keys=delta&&typeof delta==='object'?Object.keys(delta):[];
    return !!rule?.nativeMapId&&!!rule?.nativeZoneId&&!this.patch.selectors[rule.source]&&keys.length>0&&keys.every(key=>key==='noteLo'||key==='noteHi');
  }
  reconcilePreparedRuleIdentities(accepted){
    for(const rule of this.patch.rules){
      const zoneId=compiledNativeZoneId(rule),mapId=rule.nativeMapId;
      const native=accepted.rules.find(candidate=>candidate.nativeZoneId===zoneId&&(!mapId||candidate.nativeMapId===mapId));
      if(native)Object.assign(rule,{nativeMapId:native.nativeMapId,nativeZoneId:native.nativeZoneId});
      else{delete rule.nativeMapId;delete rule.nativeZoneId;}
    }
  }
  acceptSessionCommandSnapshot(session,before,fullDocument=false){
    const connection=parseSessionConnection(session.connection),accepted=patchFromSession(session);
    if(connection.instanceId!==before.instanceId||connection.attachmentId!==before.attachmentId)throw new Error('Native session command returned another connection');
    if(fullDocument)this.patch=accepted;
    else for(const rule of this.patch.rules){
      if(!rule.nativeMapId||!rule.nativeZoneId)continue;
      const native=accepted.rules.find(candidate=>candidate.nativeMapId===rule.nativeMapId&&candidate.nativeZoneId===rule.nativeZoneId);
      if(!native)throw new Error(`Native session command lost zone ${rule.nativeMapId}/${rule.nativeZoneId}`);
      Object.assign(rule,{noteLo:native.noteLo,noteHi:native.noteHi});
    }
    this.session=session;this.connection=connection;this.generation=session.document.generation;this.sourcePath=session.sourcePath;this.samplerSession=session;this.patch.params=accepted.params;
    this.parameters.accept({generation:this.generation,sequence:0,parameters:Object.values(this.patch.params).map(p=>({id:p.id,name:p.name,value:p.value}))});
    return connection;
  }
  async publishSessionCommand(method,args){
    const fullDocument=this.backendId==='juce-free'&&method!=='setInstrumentZoneKeyRange';
    return this.serial(async()=>{
      this.preparing=true;this.emit('patch',{phase:'preparing'});
      try{
        const before=this.connection,request=before.nextRequestId;
        const admitted=await this.invoke(method,before.instanceId,before.attachmentId,request,before.revision,...args);
        if(admitted?.status!=='accepted'||admitted.job_id!==request)throw new Error(`Native ${method} was not admitted`);
        const completed=await this.pollJob('getInstrumentSessionJobStatus',request);
        if(this.closed)throw new Error('Engine is closed');
        const session=await this.invoke('getSamplerSession');
        if(this.closed)throw new Error('Engine is closed');
        const connection=this.acceptSessionCommandSnapshot(session,before,fullDocument);
        if(completed.request_id!==request||connection.revision!==completed.revision||connection.nextRequestId!==nextDecimalId(request))throw new Error('Native session command returned incoherent identity or revision metadata');
        await this.startFeedback(true);
        this.emit('patch',{phase:'ready',patch:this.getPatch(),replacement:false});
      }catch(error){
        let failure=error;
        if(!this.closed)try{
          const session=await this.invoke('getSamplerSession');
          if(!this.closed)this.acceptSessionCommandSnapshot(session,this.connection,fullDocument);
        }
        catch(refreshError){failure=new Error(`${errorMessage(error)}; session refresh failed: ${errorMessage(refreshError)}`,{cause:error});}
        this.report(failure);throw failure;
      }
      finally{this.preparing=false;}
    });
  }
  updateRule(id,delta){
    if(!this.ownsKeyRangeOperation('updateRule',id,delta))return this.edit('updateRule',[id,delta]);
    const rule=this.patch.rules.find(item=>item.id===id),low=delta.noteLo??rule.noteLo,high=delta.noteHi??rule.noteHi;
    if(!Number.isInteger(low)||!Number.isInteger(high)||low<0||high>127||low>high)return Promise.reject(new Error('Invalid native mapping key range'));
    return this.publishSessionCommand('setInstrumentZoneKeyRange',[rule.nativeMapId,rule.nativeZoneId,low,high]);
  }
  undoSession(){
    if(!this.connection.history.undoCount)return Promise.resolve();
    return this.publishSessionCommand('undoInstrumentSession',[]);
  }
  get sessionHistory(){return {undoCount:this.connection?.history.undoCount??0,redoCount:this.connection?.history.redoCount??0,undoLabel:this.backendId==='juce-free'?'Edit instrument':'Change Key Range'};}
  applyDelta(changes){if(this.backendId==='juce-free')return Promise.reject(new Error('Browser history deltas are unsupported by native document history'));return this.prepared(draft=>applyEntityDelta(draft,changes));}
  beginHostGesture(id){return this.serial(async()=>{await this.parameters.command('beginGesture',id,this.generation);});}
  endHostGesture(id){return this.serial(async()=>{await this.parameters.command('endGesture',id,this.generation);});}
  async resizeEditor(mode){
    if(!this.capabilities.editorResize)throw Object.assign(new Error('Editor size is controlled by the host'),{code:'capabilityUnavailable',method:'resizeSamplerEditor'});
    try{const reply=await this.invoke('resizeSamplerEditor',mode);if(reply?.status!=='accepted')throw new Error('Native editor resize was not admitted');}
    catch(error){this.report(error);throw error;}
  }
  async setParam(id,value,gesture){
    if(!this.patch.params[id])throw new Error(`Unknown parameter: ${id}`);
    if(!Number.isFinite(value)||value<0||value>1)throw new Error('Native parameter requires normalized value 0..1');
    return this.serial(async()=>{
      const generation=this.generation;
      if(gesture==='begin')await this.parameters.command('beginGesture',id,generation);
      await this.parameters.command('setParameter',id,value,generation);
      if(generation!==this.generation||this.closed)throw new Error('Native parameter belongs to stale generation');
      this.patch.params[id].value=value;synchroniseModuleParameters(this.patch,id,value);this.emit('param',{id,value,gesture});
      if(gesture==='end')await this.parameters.command('endGesture',id,generation);
      return value;
    });
  }
  resetParam(id){return this.setParam(id,this.patch.params[id]?.default);}
  newPatch(){return this.prepared(draft=>{const next=emptyPatch();next.macros=[];next.modules=[{id:'sample',type:'Sample player',params:{mode:'once'},paramIds:{}}];if(this.backendId==='juce-free')next.voicePolicy.limit=16;Object.assign(draft,next);});}
  async loadPreset(id){if(id==='empty')return this.newPatch();if(id!=='felt-kit'&&id!=='native')throw new Error(`Unknown preset: ${id}`);const path=await this.invoke('getSamplerPresetPath');return this.prepared(()=>{}, {reloadPath:path});}
  reload(){return this.prepared(()=>{}, {reloadPath:this.sourcePath});}
  chooseSamples(){return this.backendId==='juce-free'?chooseNativeSamples(this):this.invoke('chooseSamplerFiles');}
  importInto(draft,files){
    return files.map(file=>{
      if((this.backendId==='juce-free'?file.uri!==file.resourceId:!file.uri?.startsWith('/'))||!Number.isFinite(file.duration)||file.duration<=0||!Number.isInteger(file.sampleRate)||file.sampleRate<1||!Number.isInteger(file.channels)||file.channels<1)throw new Error('Native import requires an absolute path and decoded file metadata');
      const asset={id:this.id('asset'),name:file.name,uri:file.uri,state:'loaded',cached:true,duration:file.duration,sampleRate:file.sampleRate,bits:file.bits??0,channels:file.channels,frameCount:this.backendId==='juce-free'?file.frameCount:Math.round(file.duration*file.sampleRate)};draft.assets.push(asset);return asset;
    });
  }
  importAssets(files){if(this.backendId==='juce-free')return Promise.reject(new Error('Selected sample import and mapping are unsupported by this native consumer'));return this.prepared(draft=>this.importInto(draft,files));}
  importSamples(files,options={}){
    const native=this.backendId==='juce-free';
    const prepare=(refs,assertOwner=()=>{})=>this.prepared(draft=>{
      assertOwner();
      const group=draft.nodes.find(node=>node.kind==='group');
      if(native&&((options.group&&options.group!==group?.id)||(options.groupName&&options.groupName!==(group?.name??draft.name))))throw new Error('Custom import groups are unsupported by native document history');
      const assets=this.importInto(draft,refs);
      if(options.replace){if(assets[0])structureOperations.replaceRegionAsset.call(this,draft,options.replace,assets[0].id);return {assets,rule:null};}
      return {assets,rule:assets.length?mapAssets.call(this,draft,assets.map(a=>a.id),native?{policy:'sequential',...options,group:options.group??group?.id}:options):null};
    });
    if(!native)return prepare(files);
    return withNativeFiles(this,files,async(refs,assertOwner)=>{
      const result=await prepare(refs,assertOwner),accepted=this.getPatch();
      const assets=result.assets.map(asset=>accepted.assets.find(item=>item.id===asset.id));
      const rule=result.rule?accepted.rules.find(item=>item.nativeZoneId===result.rule)?.id:null;
      if(assets.some(asset=>!asset)||(result.rule&&!rule))throw new Error('Native import result is absent from the accepted session');
      return {assets,rule};
    });
  }
  releaseSamples(files){return this.backendId==='juce-free'?releaseNativeFiles(this,files):Promise.resolve();}
  mapAssets(ids,options){if(this.backendId==='juce-free')return Promise.reject(new Error('Selected sample import and mapping are unsupported by this native consumer'));return this.prepared(draft=>mapAssets.call(this,draft,ids,options));}
  relink(){return Promise.reject(new Error('Native relinking requires file selection with decoded metadata'));}
  getPeaks(id,bins){return this.live.getPeaks(id,bins);}
  getPeaksAsync(id,bins){return this.live.getPeaksAsync(id,bins);}
  pollTelemetry(){return this.live.poll();}
  noteOn(note,velocity){if(!Number.isInteger(note)||note<0||note>127||!Number.isFinite(velocity)||velocity<=0||velocity>127)throw new Error('Invalid MIDI note or velocity');return this.live.notes.press(note,velocity/127,this.generation);}
  noteOff(note){return this.live.notes.release(note);}
  audition(){throw new Error('Arbitrary sample audition is unsupported; play its mapped MIDI note');}
  auditionSource(){throw new Error('Isolated source audition is unsupported; play its mapped MIDI note');}
  analyse(){throw new Error('Sample analysis is unsupported by the native sampler');}
  cancel(){throw new Error('Sample analysis is unsupported by the native sampler');}
  getJobs(){return [];}
  close(){
    if(this.closed)return;this.live?.close();this.parameters?.close();this.closed=true;clearInterval(this.heartbeatTimer);clearInterval(this.meterTimer);
    for(const entry of this.waits){clearTimeout(entry.timer);entry.reject(new Error('Engine is closed'));}this.waits.clear();
    for(const token of this.eventTokens)this.backend.removeEventListener?.(token);this.listeners.clear();
  }
}
for(const name of Object.keys(operations))if(name!=='updateRule')NativeEngine.prototype[name]=function(...args){return this.edit(name,args);};
