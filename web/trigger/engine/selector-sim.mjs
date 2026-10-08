/** Selection state belongs to telemetry, independent of editor undo. */
export class SelectorSim {
  constructor(runtime){this.runtime=runtime;this.positions=new Map();this.plays={};}
  reset(){this.positions.clear();this.plays={};}
  resolve(id,note,velocity,path=[]){
    const runtime=this.runtime,patch=runtime.engine.patch,selector=patch.selectors[id];
    if(!selector){const asset=patch.assets.find(a=>a.id===(patch.regions.find(r=>r.id===id)?.assetId??id));return asset?.state==='loaded'||patch.modules.some(m=>'module:'+m.id===id&&m.type==='Lush')?[id]:[];}
    if(path.includes(id))return [];
    const solo=selector.candidates.some(candidate=>candidate.solo);
    const candidates=selector.candidates.filter(c=>!c.muted&&(!solo||c.solo));if(!candidates.length)return [];
    let selected,index=0;
    const key=id+':'+note,previous=this.positions.get(key)??-1;
    if(selector.policy==='stack')selected=candidates;
    else if(selector.policy==='velocity')selected=candidates.filter((candidate,i)=>velocity>=(candidate.velLo??(i===1?selector.hardLo??96:1))&&velocity<=(candidate.velHi??(i===0?selector.softHi??95:127)));
    else{
      if(['rr','alt'].includes(selector.policy))index=(previous+1)%Math.min(candidates.length,selector.policy==='alt'?2:candidates.length);
      else if(selector.policy==='weighted'){
        const total=candidates.reduce((sum,c)=>sum+c.weight,0);if(!total)return [];
        let point=(++runtime.counter*.61803398875%1)*total;index=candidates.findIndex(c=>(point-=c.weight)<0);if(index<0)index=candidates.length-1;
      }else if(selector.policy==='random')index=candidates.length>1?(previous+1+runtime.counter++%(candidates.length-1))%candidates.length:0;
      selected=[candidates[index]];this.positions.set(key,index);
    }
    if(!selected.length)return [];
    for(const candidate of selected)this.plays[candidate.id]=(this.plays[candidate.id]??0)+1;
    runtime.selectorPos[id]={last:selected.at(-1).id,next:candidates[(index+1)%candidates.length].id,index,plays:{...this.plays}};
    return selected.flatMap(candidate=>this.resolve(candidate.id,note,velocity,[...path,id]));
  }
}
