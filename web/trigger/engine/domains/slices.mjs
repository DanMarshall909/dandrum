import {syncSliceHistory} from './slice-history.mjs';
import {clone,entity,regionRange,bounded} from '../validation.mjs';

export const sliceOperations={
  setSlices(p,assetId,markers,options={}){
    const asset=entity(p.assets,assetId);if(asset.state!=='loaded')throw new Error('Slicing requires a loaded asset');
    if(!Array.isArray(markers)||markers.some(v=>!Number.isFinite(v)||v<0||v>=1))throw new Error('Invalid slice markers');
    const starts=[...new Set([0,...markers])].sort((a,b)=>a-b);
    if(starts.length>128||starts.some((v,i)=>i&&v-starts[i-1]<.0001))throw new Error('Slice markers are too close or too numerous');
    const old=p.regions.filter(r=>r.assetId===assetId&&r.kind==='slice').sort((a,b)=>a.start-b.start),ids=new Set(old.map(r=>r.id));
    const group=options.group??p.rules.find(r=>ids.has(r.source))?.group??asset.group??p.nodes.find(n=>n.kind==='group')?.id;
    if(!group)throw new Error('Choose a group before slicing');entity(p.nodes,group);
    const from=options.startNote??old[0]?.note??24;if(!Number.isInteger(from)||from<0||from+starts.length>128)throw new Error('Slice mapping exceeds the MIDI key range');
    const regions=starts.map((start,i)=>regionRange({...clone(old[i]??{}),id:old[i]?.id??this.id('slice'),assetId,kind:'slice',name:old[i]?.name??'Slice '+(i+1),
      start,end:starts[i+1]??1,note:from+i,root:from+i,playback:old[i]?.playback??'once',fades:{in:0,out:0},loop:null}));
    p.regions=p.regions.filter(r=>!ids.has(r.id)).concat(regions);p.rules=p.rules.filter(r=>!ids.has(r.source));
    p.rules.push(...regions.map(r=>({id:this.id('rule'),group,name:r.name,source:r.id,noteLo:r.note,noteHi:r.note,root:r.note,velLo:1,velHi:127})));
    syncSliceHistory(this,p,assetId,options.method??'manual');
    return regions.map(r=>r.id);
  },
  moveSliceBoundary(p,id,start){
    const region=entity(p.regions,id),slices=p.regions.filter(r=>r.assetId===region.assetId&&r.kind==='slice').sort((a,b)=>a.start-b.start),index=slices.indexOf(region),previous=slices[index-1];
    if(!previous)throw new Error('The first marker stays at the source start');
    const value=bounded(start,previous.start+.0001,region.end-.0001);previous.end=value;region.start=value;syncSliceHistory(this,p,region.assetId);
  },
  mapSlices(p,assetId,from){
    const slices=p.regions.filter(r=>r.assetId===assetId&&r.kind==='slice').sort((a,b)=>a.start-b.start);
    if(!Number.isInteger(from)||from<0||from+slices.length>128)throw new Error('Slice mapping exceeds the MIDI key range');
    slices.forEach((r,i)=>{r.note=from+i;r.root=from+i;p.rules.filter(rule=>rule.source===r.id).forEach(rule=>Object.assign(rule,{noteLo:r.note,noteHi:r.note,root:r.note}));});syncSliceHistory(this,p,assetId);
  },
  editSlices(p,ids,delta){for(const id of ids){const region=entity(p.regions,id);if(region.kind!=='slice')throw new Error('Not a slice');Object.assign(region,regionRange({...region,...clone(delta),id}));
    if(delta.note!=null){if(!Number.isInteger(delta.note)||delta.note<0||delta.note>127)throw new Error('Invalid MIDI note');p.rules.filter(r=>r.source===id).forEach(r=>Object.assign(r,{noteLo:delta.note,noteHi:delta.note,root:delta.note}));}}for(const assetId of new Set(ids.map(id=>entity(p.regions,id).assetId)))syncSliceHistory(this,p,assetId);},
  removeSlices(p,ids){const assets=new Set(ids.map(id=>entity(p.regions,id).assetId));for(const id of ids)if(entity(p.regions,id).kind!=='slice')throw new Error('Not a slice');p.regions=p.regions.filter(r=>!ids.includes(r.id));p.rules=p.rules.filter(r=>!ids.includes(r.source));for(const assetId of assets)syncSliceHistory(this,p,assetId);},
};
