import {clone,entity} from '../validation.mjs';

/** The operation owns the derived regions, including identity and per-slice edits. */
export function syncSliceHistory(engine,patch,assetId,method='manual'){
  const slices=patch.regions.filter(r=>r.assetId===assetId&&r.kind==='slice').sort((a,b)=>a.start-b.start);
  let owner=patch.regions.find(r=>r.kind==='sample'&&r.assetId===assetId);
  if(!owner){owner={id:assetId,assetId,name:entity(patch.assets,assetId).name,kind:'sample',start:0,end:1,fades:{in:0,out:0},loop:null};patch.regions.push(owner);}
  let op=patch.history.find(op=>op.regionId===owner.id&&op.name==='Slice');
  if(!op){op={id:engine.id('operation'),regionId:owner.id,name:'Slice',on:true,config:{}};patch.history.unshift(op);}
  if(!patch.history.some(op=>op.regionId===owner.id&&op.locked))patch.history.push({id:engine.id('source'),regionId:owner.id,name:'Source',on:true,locked:true,config:{asset:assetId}});
  const ids=new Set(slices.map(r=>r.id));
  op.on=true;op.config={...op.config,count:slices.length,method,segments:clone(slices),rules:clone(patch.rules.filter(r=>ids.has(r.source))),baseStart:owner.start,baseEnd:owner.end};
}

export function initialiseSliceHistory(patch,owner){
  const op=patch.history.find(op=>op.regionId===owner.id&&op.name==='Slice');
  if(!op||op.config.segments)return;
  const segments=patch.regions.filter(r=>r.assetId===owner.assetId&&r.kind==='slice'),ids=new Set(segments.map(r=>r.id));
  Object.assign(op.config,{segments:clone(segments),rules:clone(patch.rules.filter(r=>ids.has(r.source))),baseStart:owner.start,baseEnd:owner.end,regenerate:false});
}

export function applySliceHistory(engine,patch,owner){
  const op=patch.history.find(op=>op.regionId===owner.id&&op.name==='Slice');
  const old=patch.regions.filter(r=>r.assetId===owner.assetId&&r.kind==='slice'),ids=new Set(old.map(r=>r.id));
  patch.regions=patch.regions.filter(r=>!ids.has(r.id));patch.rules=patch.rules.filter(r=>!ids.has(r.source));
  if(!op?.on)return;
  const config=op.config,templates=config.segments??old;
  const count=config.count;
  if(!Number.isInteger(count)||count<0||count>128)throw new Error('Slice count must be 0–128');
  if(!count)return;
  const group=config.rules?.[0]?.group??patch.assets.find(a=>a.id===owner.assetId)?.group;
  entity(patch.nodes,group);
  const from=config.startNote??templates[0]?.note??24;
  if(from+count>128)throw new Error('Slice mapping exceeds the MIDI key range');
  const baseStart=config.baseStart??0,baseLength=(config.baseEnd??1)-baseStart;
  const stored=!config.regenerate&&templates.length===count;
  const asset=entity(patch.assets,owner.assetId);
  if(config.regenerate&&config.method==='transients'&&count>15)throw new Error('The mock analysis provides 15 transients');
  const step=60/(asset.tempo??172)/4/asset.duration/(owner.end-owner.start);
  const sourceStarts=stored?templates.map(r=>(r.start-baseStart)/baseLength):Array.from({length:count},(_,i)=>config.method==='transients'?Math.floor(i*15/count)/16:config.method==='grid'?Math.round(i/count/step)*step:i/count);
  if(sourceStarts.some((start,i)=>start>=1||i&&start<=sourceStarts[i-1]))throw new Error('Tempo grid cannot fit that many distinct slices');
  const length=owner.end-owner.start;
  const segments=sourceStarts.map((fraction,i)=>({...(clone(templates[i])??{}),id:templates[i]?.id??engine.id('slice'),assetId:owner.assetId,
    name:templates[i]?.name??'Slice '+(i+1),kind:'slice',start:owner.start+fraction*length,
    end:owner.start+(stored?(templates[i].end-baseStart)/baseLength:sourceStarts[i+1]??1)*length,
    note:stored?templates[i].note:from+i,...(stored?{}:{root:from+i}),playback:templates[i]?.playback??'once'}));
  patch.regions.push(...segments);
  patch.rules.push(...segments.map((r,i)=>({...clone(config.rules?.find(rule=>rule.source===r.id)??{}),id:config.rules?.find(rule=>rule.source===r.id)?.id??engine.id('rule'),
    source:r.id,name:r.name,group,noteLo:r.note,noteHi:r.note,root:r.root??r.note,velLo:1,velHi:127})));
  Object.assign(config,{segments:clone(segments),rules:clone(patch.rules.filter(r=>segments.some(s=>s.id===r.source))),baseStart:owner.start,baseEnd:owner.end,regenerate:false});
}
