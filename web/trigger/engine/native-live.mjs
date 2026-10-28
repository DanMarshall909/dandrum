import {createMeterTransport} from '../../shared/meter-transport.mjs';
import {createNoteAudition} from '../../tb303/src/note-audition.mjs';

/** Own bounded native feedback independently of React rendering and editor history. */
export class NativeLive {
  constructor(engine){
    this.engine=engine;this.cache=new Map();this.pending=new Map();this.waveJobs=new Set();
    this.notes=this.createNotes();
    this.telemetry={voices:{active:null,max:engine.patch.voicePolicy.limit},notes:[],playheads:[],selectorPos:{},mod:{},meters:{},host:{},hostValues:{},voiceCountAvailable:false};
  }
  createNotes(){
    const engine=this.engine;
    this.noteConnection={instanceId:engine.connection.instanceId,attachmentId:engine.connection.attachmentId};
    const notes=createNoteAudition((name,...args)=>engine.invoke(name,...args),error=>{if(this.notes===notes)engine.report(error);});
    return notes;
  }
  resetMeter(){
    const engine=this.engine;
    if(!engine.capabilities.metering)return true;
    this.meter=createMeterTransport((name,...args)=>{
      if(engine.closed)throw new Error('Engine is closed');
      return engine.invoke(name,...args);
    },packet=>{
      if(engine.closed||packet.generation!==engine.generation||!packet.display_valid)return;
      this.telemetry={...this.telemetry,meters:{main:[...packet.display_peak]},voices:{active:null,max:engine.patch.voicePolicy.limit}};
      engine.emit('telemetry',structuredClone(this.telemetry));
    });
    return this.meter.start(engine.generation);
  }
  changed(){
    this.cache.clear();
    // A document refresh preserves each held pitch's original press generation.
    const connection=this.engine.connection;
    if(connection.instanceId!==this.noteConnection.instanceId||connection.attachmentId!==this.noteConnection.attachmentId)this.notes=this.createNotes();
    this.telemetry={...this.telemetry,voices:{active:null,max:this.engine.patch.voicePolicy.limit}};
    this.engine.emit('telemetry',structuredClone(this.telemetry));
    return this.resetMeter();
  }
  poll(){return this.meter?this.meter.tick():Promise.resolve();}
  getPeaks(id,bins){
    if(!this.engine.capabilities.sourceWaveforms)return new Float32Array();
    const key=`${this.engine.generation}|${id}|${bins}`;
    const generation=this.engine.generation;
    if(!this.cache.has(key))void this.getPeaksAsync(id,bins).catch(error=>{if(generation===this.engine.generation)this.engine.report(error);});
    return this.cache.get(key)??new Float32Array();
  }
  getPeaksAsync(id,bins){
    if(!this.engine.capabilities.sourceWaveforms)return Promise.reject(Object.assign(new Error('Source waveform unavailable'),{code:'capabilityUnavailable',method:'requestSourceWaveform'}));
    const engine=this.engine,generation=engine.generation,key=`${generation}|${id}|${bins}`;
    if(this.cache.has(key))return Promise.resolve(this.cache.get(key));
    if(this.pending.has(key))return this.pending.get(key);
    if(!Number.isInteger(bins)||bins<1||bins>4096)return Promise.reject(new Error('Invalid native waveform bucket count'));
    const source=engine.session.document.sources.find(source=>source.id===id);
    if(!source)return Promise.reject(new Error('Native waveform unavailable for this source'));
    const work=(async()=>{
      const admitted=await engine.invoke('requestSourceWaveform',id,0,bins,generation);
      if(admitted?.status!=='accepted'||admitted.generation!==generation||typeof admitted.job_id!=='string')throw new Error('Native waveform was not admitted');
      this.waveJobs.add(admitted.job_id);
      try{
        const status=await engine.pollJob('getWaveformJobStatus',admitted.job_id,'ready');
        if(engine.closed||generation!==engine.generation||status.generation!==generation)throw new Error('Native waveform belongs to a stale generation');
        if(!Array.isArray(status.result?.buckets))throw new Error('Native waveform result unavailable');
        const peaks=Float32Array.from(status.result.buckets,bucket=>Math.max(Math.abs(bucket.minimum),Math.abs(bucket.maximum)));
        this.cache.set(key,peaks);
        const asset=engine.patch.assets.find(asset=>asset.id===id);
        if(asset){asset.waveformRevision=(asset.waveformRevision??0)+1;engine.emit('asset',structuredClone(asset));}
        return peaks;
      }finally{this.waveJobs.delete(admitted.job_id);}
    })();
    this.pending.set(key,work);work.finally(()=>this.pending.delete(key)).catch(()=>{});return work;
  }
  close(){
    void this.notes.releaseAll();if(this.meter)void this.meter.setVisible(false).catch(()=>{});
    for(const id of this.waveJobs)void this.engine.invoke('cancelWaveform',id).catch(()=>{});
    this.cache.clear();
  }
}
