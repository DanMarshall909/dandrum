import {historyFor} from '../engine/domains/history.mjs';
const heading=label=>({h:true,label}),property=(label,value,unit='')=>({p:true,label,value,unit});
export function historyModel(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,owner=model.page==='slices'?model.sliceOwnerId:model.regionId;
  const region=patch.regions.find(r=>r.id===owner),asset=patch.assets.find(a=>a.id===region?.assetId);
  if(!['sample','slices'].includes(model.page)||!region||!asset){model.hist=[];model.hasHist=false;return;}
  const stored=historyFor(patch,owner),source=stored.find(op=>op.locked)??{id:'source:'+owner,regionId:owner,name:'Source',locked:true,on:true,config:{asset:asset.id}},stack=stored.length?stored:[source];
  const selected=stack.find(op=>op.id===presenter.state.histSel),seconds=value=>(value*asset.duration).toFixed(3)+' s';
  const detail=op=>op.name==='Source'?asset.name:op.name==='Trim'?seconds(op.config.start)+'–'+seconds(op.config.end):op.name==='Fade'?
    'in '+Math.round(op.config.in*asset.duration*1000)+' ms · out '+Math.round(op.config.out*asset.duration*1000)+' ms':op.name==='Loop'?
    seconds(op.config.start)+'–'+seconds(op.config.end):op.name==='Normalize'?op.config.target+' dBFS':op.name==='Slice'?op.config.count+' · '+op.config.method:'Reversed';
  const command=(label,method,...args)=>presenter.command(label,method,...args);
  const menuAt=(e,type,key)=>{const pt=presenter.winPt(e,264,230);presenter.setState({cmenu:{...pt,type,key}});};
  const move=(id,before)=>{if(id===before||source.id===id)return;const list=stack.filter(op=>op.id!==id),index=list.findIndex(op=>op.id===before);
    list.splice(index<0?Math.max(0,list.length-1):index,0,stack.find(op=>op.id===id));return command('Reorder history','reorderHistory',list.map(op=>op.id));};
  model.hist=stack.map(op=>({id:op.id,name:op.name,on:op.on,locked:!!op.locked,detail:detail(op),icon:{Source:'folder',Trim:'slice',Loop:'loop',Normalize:'variation'}[op.name]??'level',
    grip:op.locked?'':'⋮⋮',tOp:op.locked?.4:1,tTip:op.locked?'The source is always on':op.on?'Turn off':'Turn on',
    ic:op.on?'var(--dd-paper-2)':'var(--dd-paper-4)',tc:op.on?'var(--dd-paper-1)':'var(--dd-paper-4)',
    bg:selected?.id===op.id?'var(--dd-vermilion-wash)':'transparent',edge:selected?.id===op.id?'inset 2px 0 0 var(--dd-vermilion)':'none',
    select:()=>presenter.setState({histSel:selected?.id===op.id?null:op.id}),
    toggle:e=>{e.stopPropagation();if(!op.locked)command('Toggle '+op.name,'setHistory',op.id,{on:!op.on});},
    drag:e=>{if(op.locked){e.preventDefault();return;}presenter.historyDrag=op.id;e.dataTransfer.setData('text/plain',op.id);},
    dragOver:e=>{if(presenter.historyDrag)e.preventDefault();},drop:e=>{if(!presenter.historyDrag)return;e.preventDefault();e.stopPropagation();move(presenter.historyDrag,op.id);presenter.historyDrag=null;},
  }));
  model.hasHist=true;model.histCount=stack.filter(op=>op.on).length+' of '+stack.length;
  model.addHistory=e=>menuAt(e,'history-add',owner);
  model.collapseHistory=()=>{const op=selected&&!selected.locked?selected:stack.find(op=>!op.locked);if(op)return command('Collapse history','collapseHistory',op.id);};
  model.noCollapseHistory=!stack.some(op=>!op.locked);
  model.histNote=selected?'Click the operation again to show the result.':'Bottom to top. Uncheck to bypass, drag ⋮⋮ to reorder, click to edit.';
  if(model.page==='sample'&&!selected){
    model.insp={icon:asset.state==='missing'?'file-missing':'level',title:asset.name,type:'Sample',ann:'Asset + Asset Region',rows:[
      heading('Sample'),property('State',asset.state+(asset.cached?' · cached':'')),property('Format',asset.sampleRate/1000+' kHz · '+asset.bits+'-bit'),
      property('Channels',asset.channels===2?'Stereo':'Mono'),property('Used by',patch.rules.filter(r=>r.source===region.id).length+' zone(s)'),
      heading('Region'),property('Start',seconds(region.start)),property('End',seconds(region.end)),property('Loop',region.loop?seconds(region.loop.start)+'–'+seconds(region.loop.end):'Off'),property('Root',(region.root??60)+' '+presenter.nn(region.root??60)),
      heading('Analysis'),property('Pitch',model.meta.find(row=>row.k==='Pitch')?.v??'Not analysed'),property('Loudness',model.meta.find(row=>row.k==='Loudness')?.v??'Not analysed'),property('Zero crossings',asset.state==='loaded'?'Ready':'Unavailable'),...['pitch','loudness','loops'].map(kind=>({b:true,label:'Analyse '+kind,icon:'settings',disabled:asset.state!=='loaded',hint:asset.state!=='loaded'?'Load or locate the source before analysis':undefined,action:()=>presenter.startAnalysis?.(asset.id,kind)})),
      ...(asset.state==='missing'?[{b:true,label:'Search folder…',icon:'folder',action:()=>presenter.searchFolder?.(asset.id)}]:[]),{b:true,label:asset.state==='missing'?'Locate file…':'Replace…',icon:'folder',action:()=>asset.state==='missing'?presenter.replaceSource?.(asset.id):presenter.replaceSample?.()},
    ]};
    model.crumbs=[patch.name,patch.nodes.find(n=>n.id===patch.rules.find(r=>r.source===region.id)?.group)?.name??'Samples',region.name].map((label,index)=>({label,sep:index<2,c:'var(--dd-paper-2)'}));
  }
  if(selected){
    const numeric=(label,key,scale=1,min=0,max=1,unit='')=>({p:true,label,value:selected.config[key]*scale,unit,field:{value:selected.config[key]*scale,min:min*scale,max:max*scale,step:scale===1?.01:.001,
      onChange:value=>command('Edit '+selected.name+' '+label,'setHistory',selected.id,{config:{...selected.config,[key]:value/scale}})}});
    const fields=selected.name==='Trim'||selected.name==='Loop'?[numeric('Start','start',asset.duration),numeric('End','end',asset.duration)]:selected.name==='Fade'?
      [numeric('In','in',asset.duration*1000),numeric('Out','out',asset.duration*1000)]:selected.name==='Normalize'?[numeric('Target','target',1,-60,0,'dBFS')]:[property('File',asset.name),property('Length',asset.duration.toFixed(3),'s')];
    const choice=(label,key,values,fallback)=>({p:true,label,value:selected.config[key]??fallback,field:{value:selected.config[key]??fallback,options:values.map(([id,label])=>({id,label})),onChange:value=>command('Edit '+selected.name+' '+label,'setHistory',selected.id,{config:{...selected.config,[key]:value}})}});
    if(selected.name==='Trim')fields.push(choice('Snap','snap',[[true,'Zero crossings'],[false,'Off']],true));
    if(selected.name==='Fade')fields.splice(1,0,choice('In curve','inCurve',[['linear','Linear'],['exponential','Exponential']], 'exponential'));
    if(selected.name==='Fade')fields.push(choice('Out curve','outCurve',[['linear','Linear'],['exponential','Exponential']],'linear'));
    if(selected.name==='Normalize')fields.push(choice('Mode','mode',[['peak','Peak'],['rms','RMS']],'peak'),property('Gain applied',(selected.on?selected.config.target+1.2:0).toFixed(1),'dB'));
    if(selected.name==='Loop')fields.push(numeric('Crossfade','crossfade',asset.duration*1000),choice('Mode','mode',[['forward','Forward'],['ping-pong','Ping-pong'],['sustain','Sustain']],'forward'));
    if(selected.name==='Slice')fields.splice(0,fields.length,numeric('Count','count',1,1,128),choice('Method','method',[['even','Even division'],['transients','Transients'],['grid','Tempo grid'],['manual','Manual']],'even'));
    if(selected.locked)fields.splice(0,fields.length,property('File',asset.name),property('Format',asset.sampleRate/1000+' kHz · '+asset.bits+'-bit'),property('Length',asset.duration.toFixed(3),'s'),{b:true,label:'Replace…',icon:'folder',action:()=>presenter.replaceSource?.(asset.id)});
    model.insp={icon:source.id===selected.id?'folder':'settings',title:selected.name,type:'Operation',ann:'Non-destructive asset operation',rows:[heading(selected.name),...fields,{t:true,label:selected.on?'Operations apply bottom to top. The source file is never changed.':'This operation is off and is skipped.'}]};
    model.crumbs=[patch.name,asset.name,selected.name].map((label,index)=>({label,sep:index<2,c:'var(--dd-paper-2)'}));
  }
  const context=presenter.state.cmenu;
  if(context?.type==='history-add'){
    model.kmenus=[{x:context.x,y:context.y,title:'Add operation',items:['Trim','Fade','Normalize','Loop','Reverse'].map(type=>({label:type,onSelect:()=>{presenter.setState({cmenu:null});command('Add '+type,'addHistory',type,{},owner);}}))}];
  }else if(context?.type==='op'){
    const op=stack.find(item=>item.id===context.key);if(op)model.kmenus=[{x:context.x,y:context.y,title:op.name,items:[
      {label:'Edit',onSelect:()=>presenter.setState({histSel:op.id,cmenu:null})},
      {label:op.on?'Turn off':'Turn on',disabled:op.locked,onSelect:()=>{presenter.setState({cmenu:null});command('Toggle '+op.name,'setHistory',op.id,{on:!op.on});}},
      {label:'Collapse to here',disabled:op.locked,onSelect:()=>{presenter.setState({cmenu:null});command('Collapse history','collapseHistory',op.id);}},
      {separator:true},{label:'Delete operation',danger:true,disabled:op.locked,onSelect:()=>{presenter.setState({cmenu:null,histSel:null});command('Delete '+op.name,'removeHistory',op.id);}},
    ]}];
  }
}
