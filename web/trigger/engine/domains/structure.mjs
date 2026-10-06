import {syncSliceHistory} from './slice-history.mjs';
import {clone,entity,named,noteRange,regionRange} from '../validation.mjs';
import {updateRegionHistory} from './history.mjs';
export const structureOperations={
  addNode(p,parent,kind){entity(p.nodes,parent);if(!['group','sound'].includes(kind))throw new Error('Invalid node kind');const id=this.id('node');p.nodes.push({id,parent,kind,name:kind==='group'?'New group':'New sound',output:'inherit',sends:{}});return id;},
  renameNode(p,id,name){const node=entity(p.nodes,id);node.name=named(name);if(node.parent===null)p.name=node.name;},
  removeNode(p,id){
    const node=entity(p.nodes,id);if(node.parent===null)throw new Error('The instrument root cannot be deleted');
    const ids=new Set([id]);let size=0;while(size!==ids.size){size=ids.size;p.nodes.filter(n=>ids.has(n.parent)).forEach(n=>ids.add(n.id));}
    p.nodes=p.nodes.filter(n=>!ids.has(n.id));p.rules=p.rules.filter(r=>!ids.has(r.group)&&!ids.has(r.target));
    for(const key of Object.keys(p.selectors))if(ids.has(key))delete p.selectors[key];
  },
  duplicateNode(p,id){
    const original=entity(p.nodes,id);if(original.parent===null)throw new Error('The instrument root cannot be duplicated');
    const subtree=[original];for(let i=0;i<subtree.length;i++)subtree.push(...p.nodes.filter(n=>n.parent===subtree[i].id));
    const ids=new Map(subtree.map(n=>[n.id,this.id('node')]));
    p.nodes.push(...subtree.map(n=>({...clone(n),id:ids.get(n.id),parent:ids.get(n.parent)??n.parent,name:n.id===id?n.name+' copy':n.name})));
    p.rules.push(...p.rules.filter(r=>ids.has(r.group)||ids.has(r.target)).map(r=>({...clone(r),id:this.id('rule'),group:ids.get(r.group)??r.group,target:ids.get(r.target)??r.target})));
    for(const [key,value] of Object.entries(p.selectors))if(ids.has(key))p.selectors[ids.get(key)]=clone(value);
    return ids.get(id);
  },
  moveNode(p,id,parent,index){
    const node=entity(p.nodes,id),destination=entity(p.nodes,parent);if(node.parent===null||destination.kind==='sound')throw new Error('Invalid parent');
    let ancestor=destination;while(ancestor){if(ancestor.id===id)throw new Error('Cannot move a node into its own subtree');ancestor=p.nodes.find(n=>n.id===ancestor.parent);}
    node.parent=parent;p.nodes=p.nodes.filter(n=>n.id!==id);p.nodes.splice(Math.max(0,Math.min(p.nodes.length,index)),0,node);
  },
  addRule(p,rule){const id=rule.id??this.id('rule');if(p.rules.some(r=>r.id===id))throw new Error('Duplicate rule ID');entity(p.nodes,rule.group);p.rules.push(noteRange({...clone(rule),id}));return id;},
  updateRule(p,id,delta){const rule=entity(p.rules,id);Object.assign(rule,noteRange({...rule,...clone(delta),id}));},
  removeRule(p,id){entity(p.rules,id);p.rules=p.rules.filter(r=>r.id!==id);},
  setRules(p,rules){for(const rule of rules){noteRange(rule);entity(p.nodes,rule.group);}if(new Set(rules.map(r=>r.id)).size!==rules.length)throw new Error('Duplicate rule ID');p.rules=clone(rules);},
  setRegion(p,id,delta){const region=entity(p.regions,id);Object.assign(region,regionRange({...region,...clone(delta),id}));updateRegionHistory(this,p,region,delta);},
  splitSlice(p,id,at){
    const region=entity(p.regions,id);if(region.kind!=='slice'||at<=region.start||at>=region.end)throw new Error('Split must be inside a slice');
    const next={...clone(region),id:this.id('slice'),name:region.name+' 2',start:at};region.end=at;p.regions.splice(p.regions.indexOf(region)+1,0,next);
    for(const rule of p.rules.filter(r=>r.source===id))p.rules.push({...clone(rule),id:this.id('rule'),source:next.id,name:next.name});
    syncSliceHistory(this,p,region.assetId);return next.id;
  },
  mergeSlices(p,a,b){
    const first=entity(p.regions,a),next=entity(p.regions,b);
    if(first.kind!=='slice'||next.kind!=='slice'||first.assetId!==next.assetId||Math.abs(first.end-next.start)>1e-6)throw new Error('Only adjacent slices of the same asset can merge');
    first.end=next.end;p.regions=p.regions.filter(r=>r.id!==b);p.rules=p.rules.filter(r=>r.source!==b);syncSliceHistory(this,p,first.assetId);
  },
  removeSlice(p,id){const region=entity(p.regions,id);if(region.kind!=='slice')throw new Error('Not a slice');p.regions=p.regions.filter(r=>r.id!==id);p.rules=p.rules.filter(r=>r.source!==id);syncSliceHistory(this,p,region.assetId);},
};
structureOperations.removeAsset=function(p,id){entity(p.assets,id);const regionIds=new Set(p.regions.filter(r=>r.assetId===id).map(r=>r.id));p.assets=p.assets.filter(a=>a.id!==id);p.regions=p.regions.filter(r=>!regionIds.has(r.id));p.rules=p.rules.filter(r=>r.source!==id&&!regionIds.has(r.source));
  for(const selector of Object.values(p.selectors))selector.candidates=selector.candidates.filter(c=>c.id!==id&&!regionIds.has(c.id));p.history=p.history.filter(op=>!regionIds.has(op.regionId));};
structureOperations.replaceRegionAsset=function(p,id,assetId){const asset=entity(p.assets,assetId);if(asset.state!=='loaded')throw new Error('Replacement asset is unavailable');entity(p.regions,id).assetId=assetId;p.history.filter(op=>op.regionId===id&&op.name==='Source').forEach(op=>op.config.asset=assetId);};
