import {readFileSync} from 'node:fs';
import {parse} from 'yaml';

export const yaml=readFileSync(new URL('../../../examples/patches/advanced-drum-kit.yaml',import.meta.url),'utf8');
export function session(text=yaml,generation=7){
  const graph=parse(text),sources=(graph.assets?.sample_sources??[]).map(source=>({id:source.id,sampleRateHz:48000,channelCount:2,frameCount:'51000',regions:(source.regions??[]).map(region=>({id:region.id,startFrame:String(region.start_frame),endFrame:String(region.end_frame),reverse:region.reverse??false,fadeInMs:region.fade_in_ms??0,fadeOutMs:region.fade_out_ms??0,...(region.loop?{loop:{mode:region.loop.mode,startFrame:String(region.loop.start_frame),endFrame:String(region.loop.end_frame),crossfadeMs:region.loop.crossfade_ms??0}}:{})})),slices:[]}));
  const parameters=(graph.preset_surface?.parameters??[]).map(parameter=>{const port=graph.ports.find(p=>p.name===parameter.maps_to);return {id:parameter.name,name:parameter.name,normalisedValue:(port.default-port.min)/(port.max-port.min),normalisedDefaultValue:(port.default-port.min)/(port.max-port.min),minValue:port.min,maxValue:port.max,scope:'instrument'};});
  const maps=(graph.assets?.sample_maps??[]).map(map=>({id:map.id,selectionMode:map.selection_mode??'first_match',selectionSeed:String(map.selection_seed??2026),zones:(map.zones??[]).map((zone,i)=>{const [sourceId,regionId]=zone.region.split('.'),sourceIndex=sources.findIndex(s=>s.id===sourceId),regionIndex=sources[sourceIndex].regions.findIndex(r=>r.id===regionId),region=sources[sourceIndex].regions[regionIndex];return {id:zone.id??'zone-'+i,sourceIndex,regionIndex,startFrame:region.startFrame,endFrame:region.endFrame,keyLow:zone.key_range[0],keyHigh:zone.key_range[1],velocityLow:zone.velocity_range[0],velocityHigh:zone.velocity_range[1],roundRobinGroup:zone.round_robin_group??'',chokeGroup:zone.choke_group??'',weight:zone.weight??1,controlGroup:zone.control_group,gainDb:zone.gain_db,pan:zone.pan,pitchSemitones:zone.pitch_semitones};})}));
  return {yaml:text,sourcePath:'/repo/examples/patches/advanced-drum-kit.yaml',document:{generation,instrumentId:graph.instrument?.id??'untitled',sources,maps,parameters,outputBuses:[{id:'master',name:'Master',main:true,channels:['left','right']} ]}};
}

export function backend(initial=session()){
  const calls=[],events=new Map(),jobs=new Map();let current=initial,jobSequence=0;
  const functions={
    getSamplerSession:()=>current,
    setTriggerWindowTitle:name=>({status:'accepted',title:'trigger - '+name}),
    prepareSampler:(text,generation)=>{if(generation!==current.document.generation)return 'Rejected stale instrument generation';const job_id=++jobSequence;jobs.set(job_id,{text,generation});return {status:'accepted',job_id,generation};},
    getUiJobStatus:id=>{const job=jobs.get(id);current=session(job.text,job.generation+1);return {job_id:id,generation:current.document.generation,state:'completed',error:''};},
    getSamplerPresetPath:()=>'/repo/examples/patches/advanced-drum-kit.yaml',
    reloadInstrument:()=>{const job_id=++jobSequence;jobs.set(job_id,{text:yaml,generation:current.document.generation});return {status:'accepted',job_id,generation:current.document.generation};},
    setParameter:(id,value)=>{const parameter=current.document.parameters.find(parameter=>parameter.id===id);if(parameter)parameter.normalisedValue=value;return {status:'accepted',generation:current.document.generation};},
    beginGesture:()=>({status:'accepted',generation:current.document.generation}),endGesture:()=>({status:'accepted',generation:current.document.generation}),
    getParameterState:()=>({generation:current.document.generation,sequence:0,parameters:current.document.parameters.map(parameter=>({id:parameter.id,name:parameter.name,value:parameter.normalisedValue}))}),
    noteOn:()=>({status:'accepted',generation:current.document.generation}),noteOff:()=>({status:'accepted',generation:current.document.generation}),
    noteHeartbeat:()=>true,subscribeMeter:()=>true,setMeterVisible:()=>true,getMeterPacket:()=>null,ackMeterPacket:()=>true,
    requestSourceWaveform:()=>({status:'accepted',job_id:'1',generation:current.document.generation}),getWaveformJobStatus:()=>({state:'ready',generation:current.document.generation,result:{buckets:[{minimum:-.25,maximum:.75}]}}),
  };
  return {calls,functions,events,setSession:value=>{current=value;},getNativeFunction:name=>(...args)=>{calls.push([name,...args]);if(!functions[name])throw new Error('Unavailable '+name);return Promise.resolve(functions[name](...args));},addEventListener:(name,fn)=>{events.set(name,fn);return name;},removeEventListener:name=>events.delete(name)};
}
