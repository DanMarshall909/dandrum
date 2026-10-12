import {emptyPatch} from './presets.mjs';
import {patchFromSession,compileNativePatch} from './native-document.mjs';
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

const operations={...structureOperations,...instrumentOperations,...historyOperations,...sliceOperations,...candidateOperations,...routingOperations};
const unsupported=new Set(['collapseHistory','addModulator','setModulator','removeModulator','addRoute','updateRoute','removeRoute','setMacro','bindMacro','learnMidi','receiveMidi','addProcessor','moveProcessor','setProcessorParam','removeProcessor']);
/** Native preparation is the only publication boundary. Domain edits remain plain data. */
export class NativeEngine {
  constructor({backend=globalThis.window?.__JUCE__?.backend,pollMs=30,heartbeatMs=1000}={}){
    if(!backend?.getNativeFunction)throw new Error('JUCE native sampler backend unavailable');
    this.backend=backend;this.pollMs=pollMs;this.heartbeatMs=heartbeatMs;this.patch=emptyPatch();this.patch.macros=[];
    this.listeners=new Map();this.closed=false;this.sequence=0;this.pending=null;this.waits=new Set();this.eventTokens=[];
    this.pendingGeneration=0;this.preparing=false;this.reconciliation=null;
    this.native=true;this.capabilities={analysis:false,zeroCrossings:false,fx:false,modulation:false,wholeSourceAudition:false};
  }
  async initialize(){
    this.session=await this.invoke('getSamplerSession');this.patch=patchFromSession(this.session);this.generation=this.session.document.generation;
    this.sourcePath=this.session.sourcePath;this.samplerSession=this.session;this.loadedPreset='native';
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
      this.meterTimer=setInterval(()=>void this.pollTelemetry().catch(error=>this.report(error)),50);
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
      const accepted=patchFromSession(session),replacement=session.sourcePath!==this.sourcePath||JSON.stringify(parse(session.yaml))!==JSON.stringify(parse(this.session.yaml));
      this.session=session;this.generation=session.document.generation;this.sourcePath=session.sourcePath;
      if(this.pendingGeneration<=this.generation)this.pendingGeneration=0;
      if(accepted.rules.length)this.samplerSession=session;
      if(replacement)this.patch=accepted;
      else this.patch.params=accepted.params;
      await this.startFeedback(true);
      this.parameters.accept({generation:this.generation,sequence:0,parameters:Object.values(this.patch.params).map(p=>({id:p.id,name:p.name,value:p.value}))});
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
  report(error){if(!this.closed)this.emit('patch',{phase:'failed',message:error instanceof Error?error.message:String(error)});}
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
      this.emit('patch',{phase:'preparing'});
      try{
        const draft=this.getPatch(),result=edit(draft);
        if(draft.rules.some(rule=>rule.source)&&!Object.keys(draft.params).length)draft.params=patchFromSession(this.samplerSession).params;
        const text=reloadPath?null:compileNativePatch(draft,this.samplerSession),generation=this.generation;
        const admitted=await this.invoke(reloadPath?'reloadInstrument':'prepareSampler',reloadPath??text,generation);
        if(admitted?.status!=='accepted'||admitted.generation!==generation||!Number.isInteger(admitted.job_id))throw new Error('Native preparation was not admitted');
        const completed=await this.pollJob('getUiJobStatus',admitted.job_id);
        const session=await this.invoke('getSamplerSession');
        if(this.closed)throw new Error('Engine is closed');
        if(session.document?.generation<=generation||completed.generation!==session.document.generation)throw new Error('Native preparation completed with stale generation');
        const accepted=patchFromSession(session); // Validate real file/frame metadata before publication.
        this.session=session;this.generation=session.document.generation;this.sourcePath=session.sourcePath;
        if(accepted.rules.length)this.samplerSession=session;
        this.patch=reloadPath?accepted:draft;
        this.patch.params=accepted.params;
        // Native geometry is authoritative; keep stable editor identities and undo operands.
        for(const asset of this.patch.assets){const decoded=accepted.assets.find(a=>a.id===asset.id||a.uri===asset.uri);if(decoded)Object.assign(asset,{sampleRate:decoded.sampleRate,channels:decoded.channels,duration:decoded.duration,frameCount:decoded.frameCount});}
        await this.startFeedback(true);
        this.parameters.accept({generation:this.generation,sequence:0,parameters:Object.values(this.patch.params).map(p=>({id:p.id,name:p.name,value:p.value}))});
        this.emit('patch',{phase:'ready',patch:this.getPatch()});return structuredClone(result);
      }catch(error){this.report(error);throw error;}
      finally{this.preparing=false;if(this.pendingGeneration>this.generation)void this.requestGeneration(this.pendingGeneration);else this.pendingGeneration=0;}
    });
  }
  edit(name,args){
    if(unsupported.has(name))return Promise.reject(new Error(`${name} is unsupported by the native sampler`));
    if(name==='setModuleParam')return Promise.reject(new Error('Voice module configuration is unsupported; use exposed native parameters'));
    if(['setHistory','addHistory'].includes(name)&&args.some(value=>value?.config?.snap||value?.snap))return Promise.reject(new Error('Zero crossing analysis is unsupported by the native sampler'));
    return this.prepared(draft=>operations[name].call(this,draft,...args));
  }
  applyDelta(changes){return this.prepared(draft=>applyEntityDelta(draft,changes));}
  beginHostGesture(id){return this.serial(async()=>{await this.parameters.command('beginGesture',id,this.generation);});}
  endHostGesture(id){return this.serial(async()=>{await this.parameters.command('endGesture',id,this.generation);});}
  async resizeEditor(mode){
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
  newPatch(){return this.prepared(draft=>{const next=emptyPatch();next.macros=[];next.modules=[{id:'sample',type:'Sample player',params:{mode:'once'},paramIds:{}}];Object.assign(draft,next);});}
  async loadPreset(id){if(id==='empty')return this.newPatch();if(id!=='felt-kit'&&id!=='native')throw new Error(`Unknown preset: ${id}`);const path=await this.invoke('getSamplerPresetPath');return this.prepared(()=>{}, {reloadPath:path});}
  reload(){return this.prepared(()=>{}, {reloadPath:this.sourcePath});}
  chooseSamples(){return this.invoke('chooseSamplerFiles');}
  importInto(draft,files){
    return files.map(file=>{
      if(!file.uri?.startsWith('/')||!Number.isFinite(file.duration)||file.duration<=0||!Number.isInteger(file.sampleRate)||file.sampleRate<1||!Number.isInteger(file.channels)||file.channels<1)throw new Error('Native import requires an absolute path and decoded file metadata');
      const asset={id:this.id('asset'),name:file.name,uri:file.uri,state:'loaded',cached:true,duration:file.duration,sampleRate:file.sampleRate,bits:file.bits??0,channels:file.channels,frameCount:Math.round(file.duration*file.sampleRate)};draft.assets.push(asset);return asset;
    });
  }
  importAssets(files){return this.prepared(draft=>this.importInto(draft,files));}
  importSamples(files,options={}){return this.prepared(draft=>{const assets=this.importInto(draft,files);if(options.replace){if(assets[0])structureOperations.replaceRegionAsset.call(this,draft,options.replace,assets[0].id);return {assets,rule:null};}return {assets,rule:assets.length?mapAssets.call(this,draft,assets.map(a=>a.id),options):null};});}
  mapAssets(ids,options){return this.prepared(draft=>mapAssets.call(this,draft,ids,options));}
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
for(const name of Object.keys(operations))NativeEngine.prototype[name]=function(...args){return this.edit(name,args);};
