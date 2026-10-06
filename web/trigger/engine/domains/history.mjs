import {clone,entity,named,reorder,regionRange} from '../validation.mjs';
import {snapZero} from '../mock-wave.mjs';
import {initialiseSliceHistory,applySliceHistory} from './slice-history.mjs';
export const historyFor=(patch,regionId)=>patch.history.filter(op=>op.regionId===regionId||(!op.regionId&&regionId==='k60f'));
function sourceFor(engine,patch,region){
  let source=historyFor(patch,region.id).find(op=>op.locked);
  if(!source){source={id:engine.id('source'),regionId:region.id,name:'Source',on:true,locked:true,config:{asset:region.assetId}};patch.history.push(source);}
  return source;
}
export function applyHistory(patch,regionId){
  const region=entity(patch.regions,regionId),stack=historyFor(patch,regionId);
  const values={start:0,end:1,fades:{in:0,out:0},loop:null,reverse:false};
  for(const op of [...stack].reverse().filter(op=>op.on)){
    if(op.name==='Trim')Object.assign(values,{start:op.config.snap?snapZero(region.assetId,op.config.start):op.config.start,end:op.config.snap?snapZero(region.assetId,op.config.end):op.config.end});
    if(op.name==='Fade')values.fades={in:op.config.in??0,out:op.config.out??0};
    if(op.name==='Loop')values.loop=clone(op.config);
    if(op.name==='Reverse')values.reverse=!values.reverse;
  }
  const length=values.end-values.start;
  values.fades.in=Math.min(length,values.fades.in);values.fades.out=Math.min(length,values.fades.out);
  Object.assign(region,regionRange({...region,...values}));
}
export function updateRegionHistory(engine,patch,region,delta){
  if(region.kind==='slice')return;
  const fields=[['Trim',['start','end'],{start:region.start,end:region.end}],['Fade',['fades'],region.fades??{in:0,out:0}],
    ['Loop',['loop'],region.loop],['Reverse',['reverse'],{}]];
  if(!fields.some(([,keys])=>keys.some(key=>key in delta)))return;
  sourceFor(engine,patch,region);
  for(const [name,keys,config] of fields){if(!keys.some(key=>key in delta))continue;
    let op=historyFor(patch,region.id).find(op=>op.name===name);
    if(!op){op={id:engine.id('operation'),regionId:region.id,name,on:true,config:{}};patch.history.unshift(op);}
    op.config=clone(config??{});op.on=name==='Loop'?!!region.loop:name==='Reverse'?!!region.reverse:true;
  }
}
export const historyOperations={
  setHistory(p,id,delta){const op=entity(p.history,id);if(op.locked&&delta.on===false)throw new Error('Source is always on');
    const owner=op.regionId?entity(p.regions,op.regionId):null;if(owner)initialiseSliceHistory(p,owner);
    if(op.name==='Slice'&&delta.config&&(delta.config.count!==op.config.count||delta.config.method!==op.config.method))delta={...delta,config:{...op.config,...delta.config,method:delta.config.method??op.config.method,regenerate:true}};
    Object.assign(op,clone(delta),{id});if(owner){applyHistory(p,owner.id);if(p.history.some(item=>item.regionId===owner.id&&item.name==='Slice'))applySliceHistory(this,p,owner);}},
  reorderHistory(p,order){
    const requested=order.map(id=>entity(p.history,id)),owner=requested[0]?.regionId;
    const stack=owner?historyFor(p,owner):p.history;
    const next=reorder(stack,order);if(!next.at(-1)?.locked)throw new Error('Source must remain at the bottom');
    let index=0;p.history=p.history.map(op=>stack.includes(op)?next[index++]:op);if(owner){const region=entity(p.regions,owner);initialiseSliceHistory(p,region);applyHistory(p,owner);if(p.history.some(op=>op.regionId===owner&&op.name==='Slice'))applySliceHistory(this,p,region);}
  },
  addHistory(p,type,config={},regionId='k60f'){
    if(!['Trim','Fade','Normalize','Loop','Reverse','Slice'].includes(type))throw new Error('Unknown history operation');
    const region=entity(p.regions,regionId);sourceFor(this,p,region);const id=this.id('operation');
    const defaults={Trim:{start:region.start,end:region.end},Fade:region.fades??{in:0,out:0},Normalize:{target:-1},Loop:region.loop??{start:region.start,end:region.end,crossfade:0},Reverse:{},Slice:{count:8,method:'even'}};
    p.history.unshift({id,regionId,name:named(type),on:true,config:{...clone(defaults[type]),...clone(config)}});applyHistory(p,regionId);return id;
  },
  removeHistory(p,id){const op=entity(p.history,id);if(op.locked)throw new Error('Source cannot be deleted');const owner=op.regionId?entity(p.regions,op.regionId):null;if(owner)initialiseSliceHistory(p,owner);p.history=p.history.filter(item=>item.id!==id);if(owner){applyHistory(p,owner.id);if(op.name==='Slice'||p.history.some(item=>item.regionId===owner.id&&item.name==='Slice'))applySliceHistory(this,p,owner);}},
  collapseHistory(p,id){
    const op=entity(p.history,id);if(op.locked)throw new Error('Source cannot be collapsed');
    const region=entity(p.regions,op.regionId??'k60f'),source=sourceFor(this,p,region),asset=entity(p.assets,region.assetId);
    const stack=historyFor(p,region.id),index=stack.findIndex(item=>item.id===id),retained=stack.slice(0,index),assetId=this.id('rendered');
    const baked=clone(region);applyHistory({regions:[baked],history:stack.slice(index)},region.id);
    p.assets.push({...clone(asset),id:assetId,name:asset.name.replace(/\.[^.]+$/,'')+'_rendered.wav',uri:'mock://rendered/'+assetId,cached:true,
      duration:asset.duration*(baked.end-baked.start),render:{source:asset.id,start:baked.start,end:baked.end,fades:baked.fades,reverse:baked.reverse}});
    const oldAssetId=region.assetId;region.assetId=assetId;source.config={asset:assetId};
    for(const slice of p.regions.filter(r=>r.kind==='slice'&&r.assetId===oldAssetId)){slice.assetId=assetId;slice.start=Math.max(0,(slice.start-baked.start)/(baked.end-baked.start));slice.end=Math.min(1,(slice.end-baked.start)/(baked.end-baked.start));}
    for(const operation of retained.filter(op=>op.name==='Slice')){delete operation.config.segments;delete operation.config.rules;}
    const removed=new Set(stack.slice(index).filter(item=>!item.locked).map(item=>item.id));p.history=p.history.filter(item=>!removed.has(item.id));
    if(!retained.length)Object.assign(region,{start:0,end:1,fades:{in:0,out:0},loop:null,reverse:false});else applyHistory(p,region.id);
    return assetId;
  },
};
