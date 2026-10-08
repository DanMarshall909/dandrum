import {entity} from '../validation.mjs';
const patchSource=(p,id)=>p.modules.find(m=>'module:'+m.id===id&&m.type==='Lush');
const sourceExists=(p,id)=>p.regions.some(r=>r.id===id)||p.assets.some(a=>a.id===id)||!!p.selectors[id]||!!patchSource(p,id);
export const candidateOperations={
  addCandidate(p,target,source){
    const selector=p.selectors[target];if(!selector||!sourceExists(p,source))throw new Error('Choose an existing selector and source');
    if(selector.candidates.some(c=>c.id===source))throw new Error('This source is already a candidate');
    const contains=(id,path=[])=>id===target||(!path.includes(id)&&p.selectors[id]?.candidates.some(c=>contains(c.id,[...path,id])));
    if(contains(source))throw new Error('A selector cannot contain itself');
    const region=p.regions.find(r=>r.id===source),asset=p.assets.find(a=>a.id===(region?.assetId??source));
    selector.candidates.push({id:source,name:region?.name??asset?.name??patchSource(p,source)?.type??source,weight:1,muted:false,solo:false});
  },
  removeCandidate(p,target,id){const selector=p.selectors[target];if(!selector)throw new Error('Unknown selector');entity(selector.candidates,id);selector.candidates=selector.candidates.filter(c=>c.id!==id);},
  assignSource(p,target,source,policy='stack'){
    if(!sourceExists(p,source))throw new Error('Unknown source');
    const asset=p.assets.find(a=>a.id===source);if(asset&&!p.regions.some(r=>r.id===source))p.regions.push({id:source,assetId:source,name:asset.name,kind:'sample',start:0,end:1,root:60,fades:{in:0,out:0},loop:null,playback:'once'});
    if(p.selectors[target])return candidateOperations.addCandidate.call(this,p,target,source);
    let node=entity(p.nodes,target);
    if(node.kind==='group'){
      const id=this.id('sound'),note=Math.min(127,Math.max(36,...p.rules.filter(r=>r.group===target).map(r=>r.noteHi))+1);
      node={id,parent:target,name:p.regions.find(r=>r.id===source)?.name??p.assets.find(a=>a.id===source)?.name??source,kind:'sound',note,output:'inherit',sends:{}};p.nodes.push(node);
      p.rules.push({id:this.id('rule'),target:id,group:target,source,name:node.name,noteLo:note,noteHi:note,root:note,velLo:1,velHi:127});return id;
    }
    if(node.kind!=='sound')throw new Error('Drop a source on a sound or group');
    const rules=p.rules.filter(r=>r.target===target),sources=[...new Set(rules.map(r=>r.source).filter(Boolean))];
    if(!rules.length)p.rules.push({id:this.id('rule'),target,group:node.parent,source:target,name:node.name,noteLo:node.note??node.root??60,noteHi:node.note??node.root??60,root:node.note??node.root??60,velLo:1,velHi:127});
    p.selectors[target]={policy,candidates:sources.map(id=>({id,name:p.regions.find(r=>r.id===id)?.name??id,weight:1,muted:false,solo:false}))};
    if(!sources.includes(source))candidateOperations.addCandidate.call(this,p,target,source);return target;
  },
};
