import {mappingAssignments} from '../filename-mapping.mjs';
import {entity,named} from '../validation.mjs';
import {noteName} from '../presets.mjs';
import {ensureVoice} from '../voice-defaults.mjs';
import {candidateOperations} from './candidates.mjs';

export function mapAssets(patch,assetIds,options={}){
  const assets=assetIds.map(id=>entity(patch.assets,id));
  if(assets.some(a=>a.state!=='loaded'))throw new Error('Only loaded assets can be mapped');
  if(!assets.length)return null;
  ensureVoice(patch);
  if(options.target){
    for(const asset of assets){if(!patch.regions.some(r=>r.id===asset.id))patch.regions.push({id:asset.id,assetId:asset.id,name:asset.name,kind:'sample',start:0,end:1,fades:{in:0,out:0},loop:null,playback:'once',root:60});candidateOperations.assignSource.call(this,patch,options.target,asset.id);}
    return patch.rules.find(r=>r.target===options.target)?.id??null;
  }
  let group=options.group;
  if(!group){group=this.id('group');patch.nodes.push({id:group,name:named(options.groupName??'Imported'),kind:'group',parent:'root',output:'inherit',sends:{}});}
  else if(!['group','instrument'].includes(entity(patch.nodes,group).kind))throw new Error('Mapping destination must be a group');
  if(!patch.chains.some(c=>c.id===group))patch.chains.push({id:group,name:entity(patch.nodes,group).name,output:'main',modules:[]});
  const assignments=mappingAssignments(assets,options);
  const targets=new Map();
  const targetFor=row=>{
    const key=['stack','rr'].includes(options.policy)?'all':options.policy==='root-velocity'?row.root:row.id;
    if(!targets.has(key)){const id=this.id('sound');targets.set(key,id);patch.nodes.push({id,kind:'sound',parent:group,
      name:options.policy==='root-velocity'?noteName(row.root):['stack','rr'].includes(options.policy)?options.policy==='rr'?'Round robin':'Stack':row.name.replace(/\.[^.]+$/,''),root:row.root,output:'inherit',sends:{}});}
    return targets.get(key);
  };
  for(const assignment of assignments){
    entity(patch.assets,assignment.id).group=group;
    if(!patch.regions.some(r=>r.id===assignment.id))patch.regions.push({id:assignment.id,assetId:assignment.id,name:assignment.name,
      kind:'sample',start:0,end:1,fades:{in:0,out:0},loop:null,playback:'once',root:assignment.root});
  }
  const addRule=(source,row)=>{const id=this.id('rule');patch.rules.push({id,group,target:targetFor(row),source,name:row.name,noteLo:row.noteLo,noteHi:row.noteHi,velLo:row.velLo,velHi:row.velHi,root:row.root});return id;};
  if(options.policy==='rr'){
    const selector=this.id('selector');patch.selectors[selector]={policy:'rr',reset:'per-note',candidates:assignments.slice().sort((a,b)=>(a.rr??a.index)-(b.rr??b.index)).map(row=>({id:row.id,name:row.name,weight:1,muted:false,solo:false,plays:0}))};
    return addRule(selector,assignments[0]);
  }
  return assignments.map(row=>addRule(row.id,row))[0];
}
