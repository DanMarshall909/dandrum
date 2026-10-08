import {synchroniseModuleParameters} from './voice-defaults.mjs';
import {emptyPatch,feltKit} from './presets.mjs';
import {structureOperations} from './domains/structure.mjs';
import {instrumentOperations} from './domains/instrument.mjs';
import {applyEntityDelta} from './entity-delta.mjs';
import {AssetsJobs} from './assets-jobs.mjs';
import {TelemetrySim} from './telemetry-sim.mjs';
import {entity,clone,bounded} from './validation.mjs';
import {mapAssets} from './domains/import-mapping.mjs';
import {historyOperations} from './domains/history.mjs';
import {sliceOperations} from './domains/slices.mjs';
import {candidateOperations} from './domains/candidates.mjs';
import {routingOperations} from './domains/routing.mjs';

export class MockEngine {
  constructor({preparationMs=80,preset,loadMs=600,analysisMs=900}={}) {
    this.patch=preset==='felt-kit'?feltKit():emptyPatch();
    this.preparationMs=preparationMs; this.listeners=new Map(); this.log=[]; this.closed=false;this.cancelWaits=new Set();this.preparation=null;this.sequence=0;this.epoch=0;this.loadedPreset=preset??'empty';
    this.switches={failNextAnalysis:false,unsupportedNext:false};
    this.assetsJobs=new AssetsJobs(this,{loadMs,analysisMs});this.telemetry=new TelemetrySim(this);
  }
  on(event,fn) {
    if(!this.listeners.has(event)) this.listeners.set(event,new Set());
    this.listeners.get(event).add(fn);
    if(event==='telemetry')this.telemetry.start();
    return () => {this.listeners.get(event)?.delete(fn);if(event==='telemetry'&&!this.listeners.get(event)?.size)this.telemetry.stop();};
  }
  getLog(){return this.log;}
  clearLog(){this.log=[];}
  emit(event,payload) {
    if(this.closed) return;
    this.log.push({time:Date.now(),kind:'event',name:event,args:structuredClone(payload)});
    if(this.log.length>1000)this.log.splice(0,this.log.length-1000);
    for(const fn of this.listeners.get(event)||[]) fn(payload);
  }
  result(name,result,error){this.log.push({time:Date.now(),kind:error?'error':'result',name,result:error?{message:error.message}:clone(result)});if(this.log.length>1000)this.log.splice(0,this.log.length-1000);}
  call(name,args) {if(this.closed)throw new Error('Engine is closed');this.log.push({time:Date.now(),kind:'call',name,args:clone(args)});}
  id(prefix){
    const used=new Set(Object.values(this.patch).flatMap(value=>Array.isArray(value)?value.map(item=>item.id):[]));
    let id;do{id=`${prefix}-${++this.sequence}`;}while(used.has(id));return id;
  }
  edit(name,args){const operation=routingOperations[name]??candidateOperations[name]??sliceOperations[name]??historyOperations[name]??structureOperations[name]??instrumentOperations[name];return this.prepared(name,args,draft=>operation.call(this,draft,...args));}
  newPatch(){return this.prepared('newPatch',[],draft=>{Object.assign(draft,emptyPatch());this.loadedPreset='empty';this.epoch++;this.assetsJobs.cancelAll();this.telemetry.stop();});}
  reload(){return this.loadPreset(this.loadedPreset,'reload');}

  getPatch() { return structuredClone(this.patch); }
  async loadPreset(id,operation='loadPreset') {
    if(!['empty','felt-kit'].includes(id)) throw new Error(`Unknown preset: ${id}`);
    return this.prepared(operation,[id],draft=>{Object.assign(draft,id==='felt-kit'?feltKit():emptyPatch());this.loadedPreset=id;this.epoch++;this.assetsJobs.cancelAll();this.telemetry.stop();});
  }
  prepared(name,args,edit) {
    if(this.closed)return Promise.reject(new Error('Engine is closed'));
    this.call(name,args);
    const perform=async()=>{
      if(this.closed)throw new Error('Engine is closed');
      this.emit('patch',{phase:'preparing'});
      try {
        await this.wait(this.preparationMs);
        if(this.closed)throw new Error('Engine is closed');
        const draft=this.getPatch(),result=edit(draft);
        this.patch=draft;this.emit('patch',{phase:'ready',patch:this.getPatch()});
        if(['newPatch','loadPreset','reload'].includes(name)){this.telemetry.reset();}
        this.result(name,result);return structuredClone(result);
      } catch(error) {
        this.emit('patch',{phase:'failed',message:error.message});this.result(name,null,error);throw error;
      }
    };
    const pending=this.preparation?this.preparation.catch(()=>{}).then(perform):perform();
    const tracked=pending.finally(()=>{if(this.preparation===tracked)this.preparation=null;});
    this.preparation=tracked;return tracked;
  }

  setParam(id,value,gesture) {
    if(!Number.isFinite(value)) throw new Error('Parameter value must be finite');
    const param=this.patch.params[id];
    if(!param) throw new Error(`Unknown parameter: ${id}`);
    this.call('setParam',[id,value,gesture]);
    param.value=Math.max(param.min,Math.min(param.max,value));
    synchroniseModuleParameters(this.patch,id,param.value);
    this.emit('param',{id,value:param.value,gesture});this.result('setParam',param.value);
  }
  wait(ms) {
    return new Promise((resolve,reject)=>{
      const cancel=()=>{clearTimeout(timer);this.cancelWaits.delete(cancel);reject(new Error('Engine is closed'));};
      const timer=setTimeout(()=>{this.cancelWaits.delete(cancel);resolve();},ms);
      this.cancelWaits.add(cancel);
    });
  }
  close(){this.closed=true;this.assetsJobs.cancelAll();this.telemetry.stop();for(const cancel of this.cancelWaits)cancel();this.listeners.clear();}
  applyDelta(changes){return this.prepared('applyDelta',[changes],draft=>applyEntityDelta(draft,changes));}
  importAssets(files){return this.assetsJobs.importAssets(files);}
  mapAssets(ids,options){return this.prepared('mapAssets',[ids,options],draft=>mapAssets.call(this,draft,ids,options));}
  async importSamples(files,options={}){
    this.call('importSamples',[files,options]);const assets=await this.importAssets(files),loaded=assets.filter(a=>a.state==='loaded');
    if(options.replace){if(loaded[0])await this.replaceRegionAsset(options.replace,loaded[0].id);return {assets,rule:null};}
    const rule=loaded.length?await this.mapAssets(loaded.map(a=>a.id),options):null;return {assets,rule};
  }
  relink(id,uri){return this.assetsJobs.relink(id,uri);}
  getPeaks(id,bins){const result=this.assetsJobs.getPeaks(id,bins);this.result('getPeaks',{bins:result.length});return result;}
  analyse(id,kind){return this.assetsJobs.analyse(id,kind);}
  cancel(id){return this.assetsJobs.cancel(id);}
  noteOn(note,velocity){this.telemetry.noteOn(note,velocity);}
  noteOff(note){this.telemetry.noteOff(note);}
  audition(asset,region){this.telemetry.audition(asset,region);}
  auditionSource(source,note,velocity){this.telemetry.auditionSource(source,note,velocity);}
  resetParam(id){this.call('resetParam',[id]);this.setParam(id,this.patch.params[id]?.default);}
  setMacro(id,value){this.call('setMacro',[id,value]);entity(this.patch.macros,id).value=bounded(value,0,1);this.emit('param',{id,value});this.result('setMacro',value);}
  getJobs(){return Array.from(this.assetsJobs.jobs.values(),clone);}
  setMockSwitch(name,on){
    this.call('setMockSwitch',[name,on]);
    if(name==='host'){this.telemetry.host=!!on;this.telemetry.start();if(!on){this.patch.macros.forEach(m=>m.host=false);this.emit('host',{active:false});}}
    else if(name==='missing'){const asset=this.patch.assets.find(a=>a.id==='k60f')??this.patch.assets[0];if(asset){asset.state=on?'missing':'loaded';this.emit('asset',clone(asset));}}
    else if(name==='failNextAnalysis'||name==='unsupportedNext')this.switches[name]=!!on;
    else throw new Error('Unknown mock switch');
  }

  addNode(...args){return this.edit('addNode',args);}
  removeNode(...args){return this.edit('removeNode',args);}
  renameNode(...args){return this.edit('renameNode',args);}
  duplicateNode(...args){return this.edit('duplicateNode',args);}
  moveNode(...args){return this.edit('moveNode',args);}
  addRule(...args){return this.edit('addRule',args);}
  updateRule(...args){return this.edit('updateRule',args);}
  removeRule(...args){return this.edit('removeRule',args);}
  setRules(...args){return this.edit('setRules',args);}
  setRegion(...args){return this.edit('setRegion',args);}
  splitSlice(...args){return this.edit('splitSlice',args);}
  mergeSlices(...args){return this.edit('mergeSlices',args);}
  removeSlice(...args){return this.edit('removeSlice',args);}
  setSlices(...args){return this.edit('setSlices',args);}
  moveSliceBoundary(...args){return this.edit('moveSliceBoundary',args);}
  mapSlices(...args){return this.edit('mapSlices',args);}
  editSlices(...args){return this.edit('editSlices',args);}
  removeSlices(...args){return this.edit('removeSlices',args);}
  setHistory(...args){return this.edit('setHistory',args);}
  reorderHistory(...args){return this.edit('reorderHistory',args);}
  addHistory(...args){return this.edit('addHistory',args);}
  removeHistory(...args){return this.edit('removeHistory',args);}
  collapseHistory(...args){return this.edit('collapseHistory',args);}
  setSelector(...args){return this.edit('setSelector',args);}
  reorderCandidates(...args){return this.edit('reorderCandidates',args);}
  setCandidate(...args){return this.edit('setCandidate',args);}
  addCandidate(...args){return this.edit('addCandidate',args);}
  removeCandidate(...args){return this.edit('removeCandidate',args);}
  assignSource(...args){return this.edit('assignSource',args);}
  setModuleParam(...args){return this.edit('setModuleParam',args);}
  setVoicePolicy(...args){return this.edit('setVoicePolicy',args);}
  setTemplate(...args){return this.edit('setTemplate',args);}
  addRoute(...args){return this.edit('addRoute',args);}
  updateRoute(...args){return this.edit('updateRoute',args);}
  removeRoute(...args){return this.edit('removeRoute',args);}
  setModulator(...args){return this.edit('setModulator',args);}
  addModulator(...args){return this.edit('addModulator',args);}
  removeModulator(...args){return this.edit('removeModulator',args);}
  renameMacro(...args){return this.edit('renameMacro',args);}
  bindMacro(...args){return this.edit('bindMacro',args);}
  learnMidi(...args){return this.edit('learnMidi',args);}
  receiveMidi(...args){return this.edit('receiveMidi',args);}
  setOutput(...args){return this.edit('setOutput',args);}
  setSend(...args){return this.edit('setSend',args);}
  setBus(...args){return this.edit('setBus',args);}
  setChain(...args){return this.edit('setChain',args);}
  addProcessor(...args){return this.edit('addProcessor',args);}
  moveProcessor(...args){return this.edit('moveProcessor',args);}
  bypass(...args){return this.edit('bypass',args);}
  setProcessorParam(...args){return this.edit('setProcessorParam',args);}
  removeProcessor(...args){return this.edit('removeProcessor',args);}
  removeAsset(...args){return this.edit('removeAsset',args);}
  replaceRegionAsset(...args){return this.edit('replaceRegionAsset',args);}
}
