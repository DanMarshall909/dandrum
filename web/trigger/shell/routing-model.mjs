const ranges={Low:[-12,12,'dB'],High:[-12,12,'dB'],Threshold:[-60,0,'dB'],Ratio:[1,20,':1'],Attack:[.1,100,'ms'],Release:[10,1000,'ms'],Makeup:[-12,18,'dB'],Drive:[0,100,'%'],Mix:[0,100,'%'],Bits:[1,24,'bit'],Predelay:[0,200,'ms'],'Low cut':[20,1000,'Hz'],Time:[1,2000,'ms'],Feedback:[0,100,'%'],Cutoff:[20,20000,'Hz']};
export function routingModel(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,command=(label,method,...args)=>presenter.command(label,method,...args);
  const busName=id=>patch.buses.find(b=>b.id===id)?.name??'Main',effective=node=>node.output&&node.output!=='inherit'?node.output:node.parent?effective(patch.nodes.find(n=>n.id===node.parent)):'main';
  const selected=presenter.state.routingSelection??'snare',rows=[];
  const visit=(node,depth)=>{rows.push({id:node.id,name:node.name,kind:node.kind+(node.note!=null?' · note '+node.note:''),bus:node.output??'inherit',
    options:[{id:'inherit',label:'Inherit · '+busName(node.parent?effective(patch.nodes.find(n=>n.id===node.parent)):'main')},...patch.buses.map(b=>({id:b.id,label:b.name+' '+b.channels.join('/')}))],
    setBus:bus=>command('Route '+node.name,'setOutput',node.id,bus),sa:node.sends?.rev??0,sb:node.sends?.dly??0,
    setSend:(fx,value)=>command('Change '+node.name+' '+fx+' send','setSend',node.id,fx,value),select:()=>presenter.setState({routingSelection:node.id}),effective:effective(node),
    pad:depth*14,bg:node.id===selected?'var(--dd-vermilion-wash)':'transparent',edge:node.id===selected?'inset 2px 0 0 var(--dd-vermilion)':'none'});
    if(node.id!=='keys')patch.nodes.filter(n=>n.parent===node.id).forEach(n=>visit(n,depth+1));};
  patch.nodes.filter(n=>n.parent===null).forEach(n=>visit(n,0));model.routeRows=rows;
  model.busses=patch.buses.map(b=>({...b,channels:b.channels.join('/'),main:b.id==='main',feeds:rows.filter(r=>r.effective===b.id).map(r=>r.name),levels:[-90,-90]}));
  model.changeBusses=(next,id)=>{const current=next.find(b=>b.id===id);command('Change output bus','setBus',id,{channels:current.channels.split('/').map(Number),muted:!!current.muted,level:current.level??0});};
  model.outputs=patch.buses.map(b=>({id:b.id,name:b.name,note:b.channels.join('/')}));model.showRoutingGraph=!!presenter.state.routingGraph;model.toggleRoutingGraph=()=>presenter.setState({routingGraph:!presenter.state.routingGraph});
  for(const module of patch.chains.flatMap(c=>c.modules))if(!presenter.processorDefaults.has(module.id))presenter.processorDefaults.set(module.id,{...module.params});
  model.processorControls=[];
  const layer=chain=>({...chain,source:['rev','dly'].includes(chain.id)?'FX bus':'Group',detail:chain.detail??chain.modules.length+' processors',level:chain.level??0,
    modules:chain.modules.map(module=>({...module,params:Object.entries(module.params).map(([id,value],i)=>{const [min,max,unit]=ranges[id]??[0,100,''];const key=module.id+' '+id,initial=presenter.processorDefaults.get(module.id)[id]??value;model.processorControls.push({key,default:(initial-min)/(max-min),set:v=>command('Change '+module.type+' '+id,'setProcessorParam',module.id,id,min+v*(max-min))});return {id,parameterId:key,label:id,value,default:initial,min,max,unit,summary:i<2,bipolar:min<0};})}))});
  model.insertLayers=patch.chains.filter(c=>!['rev','dly'].includes(c.id)).map(layer);model.fxLayers=patch.chains.filter(c=>['rev','dly'].includes(c.id)).map(layer);
  model.selectedChain=presenter.state.selectedChain??'drums';
  model.selectChain=id=>presenter.setState({selectedChain:id});
  model.selectProcessor=id=>presenter.setState({selectedProcessor:id});
  model.moveProcessor=(id,chain,index)=>command('Move processor','moveProcessor',id,chain,index);
  model.addProcessor=chain=>presenter.setState({dialog:{kind:'add-processor',chain}});
  model.processorChoices=['EQ','Compressor','Saturate','Bitcrush','Convolution','Delay','Filter'];model.confirmProcessor=type=>{const chain=presenter.state.dialog.chain;presenter.setState({dialog:null});return command('Add '+type,'addProcessor',chain,type);};
  model.changeLayers=(next,id)=>{
    const current=next.find(c=>c.id===id),original=patch.chains.find(c=>c.id===id);if(!current||!original)return;
    if(current.output!==original.output||!!current.muted!==!!original.muted)return command('Change '+original.name+' chain','setChain',id,{output:current.output,muted:!!current.muted});
    const gone=original.modules.find(m=>!current.modules.some(next=>next.id===m.id));if(gone)return command('Remove '+gone.type,'removeProcessor',gone.id);
    for(const module of current.modules){const before=original.modules.find(m=>m.id===module.id);if(!before)continue;
      if(!!before.bypassed!==!!module.bypassed)return command('Toggle '+module.type,'bypass',module.id,!!module.bypassed);
      const changed=module.params.find(param=>param.value!==before.params[param.id]);if(changed)return command('Change '+module.type+' '+changed.label,'setProcessorParam',module.id,changed.id,changed.value);
    }
  };
  if(model.page==='routing'){
    const row=rows.find(r=>r.id===selected)??rows[0];if(row)model.insp={icon:'pan',title:row.name,type:'Routing',ann:'Output and send connections',rows:[{h:true,label:'Output'},
      {p:true,label:'Bus',value:row.bus,field:{value:row.bus,options:row.options,onChange:row.setBus}},{p:true,label:'Plugin output',value:patch.buses.find(b=>b.id===row.effective)?.channels.join('/')},
      ...[['Reverb','rev',row.sa],['Delay','dly',row.sb]].map(([label,fx,value])=>({p:true,label,value,field:{value,min:0,max:1,step:.01,onChange:v=>row.setSend(fx,v)}}))]};
  }
  if(model.page==='fx'){
    const module=patch.chains.flatMap(c=>c.modules).find(m=>m.id===presenter.state.selectedProcessor)??patch.chains.flatMap(c=>c.modules).find(m=>m.id==='comp');
    if(module){if(!presenter.processorDefaults.has(module.id))presenter.processorDefaults.set(module.id,{...module.params});const knobs=Object.entries(module.params).map(([key,value])=>{const [min,max,unit]=ranges[key]??[0,100,''];return {key:module.id+' '+key,label:key,v:(value-min)/(max-min),default:((presenter.processorDefaults.get(module.id)[key]??value)-min)/(max-min),t:value.toFixed(Number.isInteger(value)?0:1)+' '+unit,bi:min<0,parse:text=>{const n=Number.parseFloat(String(text).replace('−','-'));return Number.isFinite(n)?(n-min)/(max-min):null;},set:v=>command('Change '+module.type+' '+key,'setProcessorParam',module.id,key,min+v*(max-min))};});
      model.insp={icon:'settings',title:module.type,type:'Processor',ann:'DSP module',rows:[{knobs:true,list:knobs},...(module.type==='Compressor'?[{h:true,label:'Gain reduction'},{p:true,label:'Now',value:'0.0',unit:'dB',ro:true},{p:true,label:'Detector',value:'Peak'},{p:true,label:'Sidechain',value:'Self'}]:[]),{b:true,label:module.bypassed?'Enable processor':'Bypass',icon:'reset',action:()=>command('Toggle '+module.type,'bypass',module.id,!module.bypassed)}]};}
  }
  if(['routing','fx'].includes(model.page))model.crumbs=[patch.name,model.page==='fx'?'Effects':'Routing',model.insp.title].map((label,i)=>({label,sep:i<2,c:'var(--dd-paper-2)'}));
}
