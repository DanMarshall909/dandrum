import {parse,stringify} from 'yaml';
import {emptyPatch} from './presets.mjs';

const clone=value=>structuredClone(value);
const readable=id=>id.replace(/[_.-]+/g,' ');
const unsupported=feature=>{throw new Error(`${feature} is unsupported by the native sampler`);};
const finite=(value,name)=>{if(!Number.isFinite(value))throw new Error(`Invalid ${name}`);return value;};
export function compiledNativeZoneId(rule){
  return rule.nativeMapId&&rule.nativeZoneId
    &&rule.id===`${rule.nativeMapId}.${rule.nativeZoneId}`?rule.nativeZoneId:rule.id;
}
export function absoluteSamplePath(path,sourcePath){
  if(typeof path!=='string'||!path||path.includes('://'))throw new Error('Native samples require an absolute file URI');
  if(path.startsWith('/'))return path;
  if(!sourcePath?.startsWith('/'))throw new Error('Native sample source provenance is unavailable');
  const parts=sourcePath.slice(0,sourcePath.lastIndexOf('/')).split('/');
  for(const part of path.split('/')){if(part==='..')parts.pop();else if(part!=='.'&&part)parts.push(part);}
  return parts.join('/');
}

function relativeSamplePath(uri,sourcePath){
  const root=sourcePath?.slice(0,sourcePath.lastIndexOf('/'));
  if(!root?.startsWith('/')||!uri.startsWith(root+'/'))throw new Error('Native sample is outside the instrument authoring root; choose a staged sample');
  const relative=uri.slice(root.length+1);
  if(!relative||relative.split('/').some(part=>!part||part==='.'||part==='..')||relative.includes('\\'))throw new Error('Native sample path must be canonical beneath the authoring root');
  return relative;
}

/** Project prepared frame bounds rather than inventing duration or sample metadata. */
export function patchFromSession(session){
  const {document,sourcePath}=session,graph=parse(session.yaml);
  if(!document||!Number.isInteger(document.generation)||!graph)throw new Error('Native sampler session is unavailable');
  const patch=emptyPatch();patch.macros=[];patch.id=document.instrumentId;patch.name=graph.metadata?.name??document.instrumentId;
  patch.nodes[0].name=patch.name;
  patch.modules=[{id:'sample',type:'Sample player',params:{mode:'once'},paramIds:{}}];
  const player=graph.modules?.find(module=>module.type==='sample_map_player');
  patch.voicePolicy.limit=player?.static?.max_voices??16;patch.voicePolicy.steal=player?.static?.voice_steal??'oldest';
  for(const param of document.parameters??[]){
    patch.params[param.id]={id:param.id,name:param.name,value:param.normalisedValue,default:param.normalisedDefaultValue,min:0,max:1,unit:'normalised',physicalMin:param.minValue,physicalMax:param.maxValue,nativeScope:param.scope,nativeControlGroup:param.controlGroup};
    const label={pitch_ratio:'Tune',start_offset:'Start',level:'Gain',pan:'Pan'}[param.id.split('.').at(-1)];
    if(label&&param.scope==='instrument'&&!patch.modules[0].paramIds[label])patch.modules[0].paramIds[label]=param.id;
  }
  for(const source of document.sources??[]){
    const declared=graph.assets?.sample_sources?.find(item=>item.id===source.id);
    if(!declared)throw new Error(`Native source ${source.id} has no file provenance`);
    const frameCount=Number(source.frameCount),duration=frameCount/source.sampleRateHz;
    if(!Number.isSafeInteger(frameCount)||frameCount<1||!Number.isFinite(duration)||duration<=0)throw new Error('Invalid native source geometry');
    const uri=absoluteSamplePath(declared.path??declared.resource?.uri,sourcePath);
    patch.assets.push({id:source.id,name:uri.split('/').at(-1),uri,state:'loaded',cached:true,duration,sampleRate:source.sampleRateHz,bits:0,channels:source.channelCount,frameCount});
    for(const region of source.regions??[]){
      patch.regions.push({id:`${source.id}.${region.id}`,nativeId:region.id,assetId:source.id,name:readable(region.id),kind:'sample',start:Number(region.startFrame)/frameCount,end:Number(region.endFrame)/frameCount,root:region.rootNote,reverse:region.reverse??false,gain:region.gainDb??0,pan:region.pan??0,fades:{in:(region.fadeInMs??0)/(duration*1000),out:(region.fadeOutMs??0)/(duration*1000)},playback:region.loop?({forward:'loop',ping_pong:'pp',sustain:'sus'}[region.loop.mode]??'loop'):'once',loop:region.loop?{mode:region.loop.mode,start:Number(region.loop.startFrame)/frameCount,end:Number(region.loop.endFrame)/frameCount,crossfade:(region.loop.crossfadeMs??0)/(duration*1000)}:null});
    }
  }
  for(const map of document.maps??[]){
    patch.nodes.push({id:map.id,name:readable(map.id),kind:'group',parent:'root',output:'inherit',sends:{}});
    const grouped=new Map();
    for(const zone of map.zones??[]){
      const source=document.sources[zone.sourceIndex],region=source?.regions[zone.regionIndex];if(!region)throw new Error('Invalid native zone source');
      const regionId=`${source.id}.${region.id}`,target=`${map.id}.sound.${zone.controlGroup??zone.keyLow}`;
      if(!patch.nodes.some(node=>node.id===target))patch.nodes.push({id:target,name:readable(zone.id),kind:'sound',parent:map.id,note:zone.keyLow,output:'inherit',sends:{},nativeControlGroup:zone.controlGroup});
      const key=zone.roundRobinGroup
        ? (zone.roundRobinGroup.startsWith(`${map.id}.`)?zone.roundRobinGroup:`${map.id}.${zone.roundRobinGroup}`)
        : null;
      if(key){
        const first=grouped.get(key);
        if(first&&['keyLow','keyHigh','velocityLow','velocityHigh','controlGroup'].some(field=>first[field]!==zone[field]))unsupported('Round robin candidates with different mapping bounds');
        if(!grouped.has(key)){
          grouped.set(key,zone);
          patch.selectors[key]={policy:map.selectionMode==='random_weighted'?'weighted':'rr',reset:'per-note',candidates:[]};
        }
        const selector=patch.selectors[key];
        const duplicate=selector.candidates.some(candidate=>candidate.id===regionId);
        selector.candidates.push({id:duplicate?`${map.id}.${zone.id}`:regionId,...(duplicate?{nativeSource:regionId}:{}),name:readable(zone.id),nativeZoneId:zone.id,weight:zone.weight??1,muted:false,solo:false,gain:zone.gainDb??0,pan:zone.pan??0,tune:zone.pitchSemitones??0,nativeChokeGroup:zone.chokeGroup});
      }
      if(key&&grouped.get(key)!==zone)continue;
      const root=patch.regions.find(item=>item.id===regionId)?.root??zone.keyLow;
      patch.rules.push({id:`${map.id}.${zone.id}`,nativeMapId:map.id,nativeZoneId:zone.id,group:map.id,target,name:readable(zone.id),source:key??regionId,noteLo:zone.keyLow,noteHi:zone.keyHigh,velLo:zone.velocityLow,velHi:zone.velocityHigh,root,nativeInitialRoot:root,nativeControlGroup:zone.controlGroup,nativeChokeGroup:zone.chokeGroup,nativeGain:zone.gainDb,nativePan:zone.pan,nativeTune:zone.pitchSemitones});
    }
  }
  return patch;
}

function validateCapabilities(patch){
  if(patch.modules.some(module=>module.type!=='Sample player'||module.bypassed))unsupported('Voice modules and bypass');
  if(patch.modules.some(module=>module.params.mode&&module.params.mode!=='once'))unsupported('Voice playback modes');
  if(patch.modulators.length||patch.routes.length||patch.macros.some(macro=>macro.bindings?.length||macro.value))unsupported('Modulation and macros');
  if(patch.chains.some(chain=>chain.modules.length||chain.muted||chain.level)||patch.buses.some(bus=>bus.id!=='main'||bus.muted||bus.level))unsupported('FX and additional output buses');
  if(patch.nodes.some(node=>(node.output&& !['inherit','main'].includes(node.output))||Object.values(node.sends??{}).some(Boolean)))unsupported('Output routing and sends');
  if(patch.history.some(op=>op.on&&(op.name==='Normalize'||op.config?.snap)))unsupported('Normalization and zero crossing analysis');
  if(patch.midiLearn)unsupported('MIDI learn');
  const policy=patch.voicePolicy;
  if(policy.mode!=='poly'||policy.glide||policy.exclusiveGroup||policy.sameNote!=='retrigger')unsupported('Mono, legato, glide and same note policies');
  if(!Number.isInteger(policy.limit)||policy.limit<1||policy.limit>128||!['oldest','quietest','reject_new'].includes(policy.steal))unsupported('Voice allocation policy');
}

/** A validated sampler graph is compiled before any native transaction is admitted. */
export function compileNativePatch(patch,session){
  validateCapabilities(patch);
  const graph=clone(parse(session.yaml));
  if(!graph.modules?.some(module=>module.type==='sample_map_player'))throw new Error('Native sampler graph has no sample map player');
  const publicIds=new Set((graph.preset_surface?.parameters??[]).map(parameter=>parameter.name));
  if(Object.keys(patch.params).some(id=>!publicIds.has(id)))unsupported('Controls absent from the native parameter surface');
  graph.metadata={...graph.metadata,name:patch.name};
  const sourceIds=new Map(),regionRefs=new Map(),compiledRegions=new Map();
  const sources=patch.assets.map(asset=>{
    if(asset.state!=='loaded'||!asset.uri.startsWith('/')||asset.render)throw new Error('Native sampler requires real decoded sample assets');
    const id=asset.id.replace(/[^a-zA-Z0-9_-]/g,'_');if([...sourceIds.values()].includes(id))throw new Error('Duplicate native source ID');sourceIds.set(asset.id,id);
    const frames=asset.frameCount??Math.round(asset.duration*asset.sampleRate);
    if(!Number.isSafeInteger(frames)||frames<1)throw new Error('Invalid native sample frame count');
    const regions=patch.regions.filter(region=>region.assetId===asset.id).map(region=>{
      const regionId=region.nativeId??region.id.replace(/[^a-zA-Z0-9_-]/g,'_');regionRefs.set(region.id,`${id}.${regionId}`);
      const frame=value=>Math.round(finite(value,'region frame')*frames),milliseconds=value=>finite(value,'fade duration')*asset.duration*1000;
      const result={id:regionId,start_frame:frame(region.start),end_frame:frame(region.end),reverse:!!region.reverse,fade_in_ms:milliseconds(region.fades?.in??0),fade_out_ms:milliseconds(region.fades?.out??0)};
      if(result.start_frame<0||result.end_frame>frames||result.end_frame<=result.start_frame)throw new Error('Invalid native sample region');
      if(region.root!=null)result.root_note=region.root;if(region.gain)result.gain_db=region.gain;if(region.pan)result.pan=region.pan;
      if(region.tune||region.choke)unsupported('Per-region tuning and choke; use candidate controls');
      if(region.loop){
        const mode=({loop:'forward',pp:'ping_pong',sus:'sustain'}[region.playback]??region.loop.mode??'forward');if(mode!=='forward')unsupported('Ping pong and sustain loops');
        result.loop={mode,start_frame:frame(region.loop.start),end_frame:frame(region.loop.end),crossfade_ms:milliseconds(region.loop.crossfade??0)};
      }
      else if(region.playback&&region.playback!=='once')unsupported('Gated region playback');
      compiledRegions.set(`${id}.${regionId}`,result);
      return result;
    });
    // An unassigned library file still needs a prepared region for real waveform decoding.
    if(!regions.length)regions.push({id:'whole',start_frame:0,end_frame:frames});
    return {id,path:relativeSamplePath(asset.uri,session.sourcePath),regions};
  });
  const modes=new Set(),zones=[];
  function expand(source,path=[],controls={}){
    if(path.includes(source))throw new Error('Native selector cycle');
    const selector=patch.selectors[source];
    if(!selector){const ref=regionRefs.get(source)??regionRefs.get(patch.regions.find(region=>region.assetId===source)?.id);if(!ref)throw new Error(`Native mapping source unavailable: ${source}`);return [{region:ref,...controls}];}
    if(path.length)unsupported('Nested selector policies');
    const active=selector.candidates.filter(candidate=>!candidate.muted&&(!selector.candidates.some(c=>c.solo)||candidate.solo));
    if(['stack','velocity'].includes(selector.policy)&&active.length>1)unsupported('Stack and velocity crossfade selectors');
    if(!['stack','rr','weighted','random'].includes(selector.policy))unsupported('Selector policy');
    if(selector.policy!=='stack')modes.add(['weighted','random'].includes(selector.policy)?'random_weighted':'round_robin');
    return active.flatMap(candidate=>{
      if(candidate.output&&!['inherit','main'].includes(candidate.output))unsupported('Candidate output routing');
      if(candidate.velLo!=null||candidate.velHi!=null)unsupported('Per candidate velocity windows');
      return expand(candidate.nativeSource??candidate.id,[...path,source],{...controls,round_robin_group:source,nativeZoneId:candidate.nativeZoneId,weight:selector.policy==='random'?1:candidate.weight??1,gain_db:candidate.gain??0,pan:candidate.pan??0,pitch_semitones:candidate.tune??0,choke_group:candidate.nativeChokeGroup});
    });
  }
  for(const rule of patch.rules){
    if(!rule.source)continue;
    const target=patch.nodes.find(node=>node.id===rule.target);
    for(const [index,candidate] of expand(rule.source).entries()){
      // Original drum maps may intentionally omit root_note (fixed-pitch hits).
      // Explicit mapping root edits get their own region, preserving shared sources.
      const rootChanged=rule.root!=null&&(rule.nativeInitialRoot==null||rule.root!==rule.nativeInitialRoot);
      if(rootChanged){
        if(!Number.isInteger(rule.root)||rule.root<0||rule.root>127)throw new Error('Invalid native mapping root');
        const original=compiledRegions.get(candidate.region);
        if(original.root_note!==rule.root){
          const source=sources.find(source=>candidate.region.startsWith(source.id+'.'));
          const id=original.id+'__root_'+rule.root;
          if(!source.regions.some(region=>region.id===id))source.regions.push({...clone(original),id,root_note:rule.root});
          candidate.region=`${source.id}.${id}`;
        }
      }
      const nativeZoneId=compiledNativeZoneId(rule);
      const {nativeZoneId:origin,...properties}=candidate;
      const zone={id:origin??(index?`${nativeZoneId}_${index}`:nativeZoneId),region:candidate.region,key_range:[rule.noteLo,rule.noteHi],velocity_range:[rule.velLo,rule.velHi]};
      Object.assign(zone,{gain_db:rule.nativeGain,pan:rule.nativePan,pitch_semitones:rule.nativeTune,choke_group:rule.nativeChokeGroup,control_group:target?.nativeControlGroup??rule.nativeControlGroup},properties);
      for(const key of Object.keys(zone))if(zone[key]==null||zone[key]==='')delete zone[key];
      zones.push(zone);
    }
  }
  if(modes.size>1)unsupported('Mixed random and round robin maps');
  if(!zones.length){
    // An unconnected summing input is exact zero audio in the real graph engine.
    graph.modules=[{id:'silence',type:'audio_mixer',static:{channels:2}}];graph.connections=[];
    graph.ports=[{name:'master',direction:'output',signal:'audio',channels:2,maps_from:'silence.mix'}];
    delete graph.preset_surface;delete graph.assets;
    if(sources.length)graph.assets={sample_sources:sources};
    return stringify(graph);
  }
  for(let i=0;i<zones.length;i++)for(let j=i+1;j<zones.length;j++){
    const a=zones[i],b=zones[j],overlap=a.key_range[0]<=b.key_range[1]&&b.key_range[0]<=a.key_range[1]&&a.velocity_range[0]<=b.velocity_range[1]&&b.velocity_range[0]<=a.velocity_range[1];
    if(overlap&&(!a.round_robin_group||a.round_robin_group!==b.round_robin_group))unsupported('Overlapping stacked sample rules');
  }
  const oldMap=graph.assets?.sample_maps?.[0],mapId=oldMap?.id??'trigger';
  graph.assets={sample_sources:sources,sample_maps:[{id:mapId,selection_seed:oldMap?.selection_seed??2026,selection_mode:[...modes][0]??'first_match',zones}]};
  for(const module of graph.modules)if(module.type==='sample_map_player')module.static={...module.static,sample_map:mapId,max_voices:patch.voicePolicy.limit,voice_steal:patch.voicePolicy.steal};
  for(const parameter of graph.preset_surface?.parameters??[]){
    const value=patch.params[parameter.name]?.value,port=graph.ports?.find(item=>item.name===parameter.maps_to);
    if(value!=null&&port){finite(value,'parameter');if(value<0||value>1)throw new Error('Invalid normalized native parameter');port.default=port.min+value*(port.max-port.min);}
  }
  return stringify(graph);
}
