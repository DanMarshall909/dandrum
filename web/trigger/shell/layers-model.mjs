export function selectedSelector(presenter){
  const patch=presenter.props.store.getSnapshot().patch,id=presenter.state.selectorTarget??presenter.state.selection?.id;
  const target=patch.selectors[id]?id:presenter.state.st==='rr'&&patch.selectors['snare-hard']?'snare-hard':patch.selectors.snare?'snare':Object.keys(patch.selectors)[0];
  return {patch,target,selector:patch.selectors[target]};
}
export function layersModel(presenter,model){
  const {patch,target,selector:accepted}=selectedSelector(presenter);if(!accepted){model.isLayers=false;return;}
  const selector=presenter.state.selectorDraft?.target===target?{...accepted,...presenter.state.selectorDraft}:accepted;
  const set=(label,delta)=>presenter.command(label,'setSelector',target,delta),candidateEdit=(id,label,delta)=>presenter.command('Change candidate '+label,'setCandidate',target,id,delta);
  const modes=[['stack','Stack'],['velocity','Velocity'],['rr','Round robin'],['alt','Alternate'],['random','Random'],['weighted','Weighted']];
  model.onLayerDrop=e=>{if(e.dataTransfer.files.length){e.preventDefault();e.stopPropagation();presenter.queueImport(e.dataTransfer.files,{target});return;}
    const source=presenter.state.dragSrc;if(source){e.preventDefault();e.stopPropagation();presenter.command('Add source candidate','addCandidate',target,source.id);presenter.setState({dragSrc:null});}};
  model.selectorId=target;model.selectorPolicy=selector.policy;
  model.selModes=modes.map(([id,label])=>({id,label,sel:id===selector.policy,go:()=>set('Change selector mode',{policy:id})}));
  model.modeDesc={stack:'All samples play on every hit.',velocity:'Velocity ranges may overlap to crossfade.',rr:'One sample per hit, in order. Each note keeps its own position.',alt:'Two samples, swapping on every hit.',random:'One sample per hit at random; never the same twice in a row.',weighted:'One sample per hit at random, in proportion to each weight.'}[selector.policy];
  model.isVel=selector.policy==='velocity';model.isRR=['rr','alt','random','weighted'].includes(selector.policy);model.weightHead=selector.policy==='weighted'?'Weight':'Share';
  model.layerCtx=target==='snare-hard'?'Snare › Hard · note 38 D2 · vel 96–127':(patch.nodes.find(n=>n.id===target)?.name??'Selector')+' · '+selector.candidates.length+' candidates';
  model.resetCycle=selector.reset==='transport'?'transport':selector.reset==='bar'?'bar':'never';model.setCycleReset=reset=>set('Change cycle reset',{reset:reset==='never'?'per-note':reset});
  const lo=selector.hardLo??96,hi=selector.softHi??95,width=Math.max(0,hi-lo+1),mid=(lo+hi)/2;
  model.vXfW=width;model.vXfText=width?lo+'–'+hi+' · '+width+' steps':'Off · hard split at '+lo;model.vNoXf=!width;
  model.setVelW=value=>{const w=Math.max(0,Math.min(60,Math.round(value))),hardLo=Math.max(2,Math.min(127-w,Math.round(mid-(w-1)/2)));set('Change velocity crossfade',{hardLo,softHi:hardLo+w-1});};
  model.velNoXf=()=>set('Remove velocity crossfade',{hardLo:Math.round(mid)+1,softHi:Math.round(mid)});
  model.vCurve=selector.curve==='linear'?'lin':'eq';model.setVelCurve=value=>set('Change crossfade curve',{curve:value==='lin'?'linear':'equal-power'});
  model.onVelDown=e=>{
    const handle=e.target.closest('[data-vh]');if(!handle)return;e.preventDefault();const box=e.currentTarget.getBoundingClientRect(),start=(e.clientX-box.left)/box.width*126+1,kind=handle.dataset.vh;
    let delta=null;const move=event=>{const v=Math.round((event.clientX-box.left)/box.width*126+1),d=v-start;
      delta=kind==='l'?{hardLo:Math.max(2,Math.min(hi+1,v))}:kind==='r'?{softHi:Math.max(lo-1,Math.min(126,v))}:{hardLo:Math.round(Math.max(2,Math.min(127-width,lo+d))),softHi:Math.round(Math.max(2,Math.min(127-width,lo+d)))+width-1};
      presenter.setState({selectorDraft:{target,...delta}});};
    const cleanup=()=>{window.removeEventListener('pointermove',move);window.removeEventListener('pointerup',finish);window.removeEventListener('pointercancel',cancel);presenter.velocityCleanup=null;};
    const cancel=()=>{cleanup();presenter.setState({selectorDraft:null});},finish=()=>{cleanup();if(delta)set('Move velocity crossfade',delta).finally(()=>presenter.setState({selectorDraft:null}));};
    presenter.velocityCleanup?.();presenter.velocityCleanup=cancel;window.addEventListener('pointermove',move);window.addEventListener('pointerup',finish,{once:true});window.addEventListener('pointercancel',cancel,{once:true});
  };
  const chosen=selector.candidates.find(c=>c.id===presenter.state.candidateSelection)??selector.candidates[1]??selector.candidates[0],total=selector.candidates.reduce((n,c)=>n+c.weight,0);
  model.selectedCandidate=chosen?.id;
  const choose=id=>presenter.setState({candidateSelection:id});
  const reorder=(id,before)=>{const ids=selector.candidates.map(c=>c.id).filter(item=>item!==id);ids.splice(Math.max(0,ids.indexOf(before)),0,id);presenter.command('Reorder candidates','reorderCandidates',target,ids);};
  model.cands=selector.candidates.map((c,i)=>{
    const region=patch.regions.find(r=>r.id===c.id),asset=patch.assets.find(a=>a.id===(region?.assetId??c.id)),nested=patch.selectors[c.id],share=selector.policy==='weighted'?(total?c.weight/total:0):1/selector.candidates.length;
    return {id:c.id,ord:i+1,name:c.name,file:asset?.name??nested?.candidates.length+' alternates · '+nested?.policy,vel:selector.policy==='velocity'?(c.velLo??(i===1?lo:1))+'–'+(c.velHi??(i===0?hi:127)):'1–127',
      w:selector.policy==='weighted'?c.weight.toFixed(1):selector.policy==='velocity'?'—':Math.round(share*100)+'%',wpct:share*100,gain:(c.gain??0).toFixed(1)+' dB',pan:c.pan?Math.round(Math.abs(c.pan)*100)+(c.pan<0?' L':' R'):'C',tune:(c.tune??0)+' st',out:patch.buses.find(b=>b.id===c.output)?.name??'Inherit',
      muted:c.muted,solo:c.solo,mc:c.muted?'var(--dd-vermilion)':'var(--dd-paper-4)',dot:'var(--dd-paper-4)',bg:c.id===chosen?.id?'var(--dd-vermilion-wash)':'transparent',edge:c.id===chosen?.id?'inset 2px 0 0 var(--dd-vermilion)':'none',
      select:()=>choose(c.id),toggleMute:e=>{e.stopPropagation();candidateEdit(c.id,'mute',{muted:!c.muted});},toggleSolo:e=>{e.stopPropagation();candidateEdit(c.id,'solo',{solo:!c.solo});},
      drag:e=>{presenter.candidateDrag=c.id;e.dataTransfer.setData('text/plain',c.id);},over:e=>{if(presenter.candidateDrag)e.preventDefault();},drop:e=>{if(!presenter.candidateDrag)return;e.preventDefault();e.stopPropagation();reorder(presenter.candidateDrag,c.id);presenter.candidateDrag=null;},
    };
  });
  model.rrSeq=selector.candidates.map(c=>({id:c.id,label:c.name,tag:'',bg:'var(--dd-ink-4)',bd:'var(--dd-line-2)',tc:'var(--dd-paper-3)'}));
  model.addCandidate=e=>{const pt=presenter.winPt(e,264,240);presenter.setState({cmenu:{...pt,type:'candidate-add',key:target}});};
  const menu=presenter.state.cmenu;
  if(menu?.type==='candidate-add')model.kmenus=[{x:menu.x,y:menu.y,title:'Add sample to '+target,items:patch.regions.filter(r=>!selector.candidates.some(c=>c.id===r.id)).map(r=>({label:r.name,onSelect:()=>{presenter.setState({cmenu:null});presenter.command('Add candidate','addCandidate',target,r.id);}}))}];
  if(model.page!=='layers')return;
  presenter.candidateKeyboard={step:direction=>{const at=selector.candidates.indexOf(chosen);choose(selector.candidates[Math.max(0,Math.min(selector.candidates.length-1,at+direction))]?.id);},remove:()=>chosen&&presenter.command('Delete candidate','removeCandidate',target,chosen.id)};
  const h=label=>({h:true,label}),p=(label,value,field)=>({p:true,label,value,field}),numeric=(label,key,min,max,unit)=>({...p(label,chosen?.[key]??0,{value:chosen?.[key]??0,min,max,step:.01,onChange:value=>candidateEdit(chosen.id,label,{[key]:value})}),unit});
  model.insp={icon:'alternate',title:chosen?.name??'No candidates',type:'Candidate',ann:'Selector candidate',rows:chosen?[h('Sample'),p('File',model.cands.find(c=>c.id===chosen.id)?.file),h('Selection'),p('Order',selector.candidates.indexOf(chosen)+1),numeric('Weight','weight',0,100),numeric('Gain','gain',-60,12,'dB'),numeric('Pan','pan',-1,1),numeric('Tune','tune',-48,48,'st'),
    {b:true,label:'Preview',icon:'one-shot',action:()=>{const region=patch.regions.find(r=>r.id===chosen.id);if(region)presenter.props.store.audition(region.assetId,region);else presenter.props.store.auditionSource(chosen.id,patch.rules.find(r=>r.source===chosen.id)?.root??38,100);}},
    {b:true,label:'Delete candidate',icon:'minus',action:()=>presenter.candidateKeyboard.remove()}]:[{t:true,label:'Add a sample to this selector.'}]};
  model.crumbs=[patch.name,'Layers',chosen?.name??target].map((label,i)=>({label,sep:i<2,c:'var(--dd-paper-2)'}));
}
