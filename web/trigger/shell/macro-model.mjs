export function macroModel(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,menu=presenter.state.cmenu,macro=patch.macros.find(m=>m.id===presenter.state.selectedMacro);
  const command=(label,method,...args)=>presenter.command(label,method,...args),close=()=>presenter.setState({cmenu:null});
  model.selectMacro=id=>{presenter.go('voice');presenter.setState({selectedMacro:id});};
  model.openMacroMenu=(e,id)=>{e.preventDefault();e.stopPropagation();const pt=presenter.winPt(e,264,300);presenter.setState({cmenu:{...pt,type:'macro',key:id},kmenu:null,dmenu:null});};
  const bindMenu=(e,id)=>{const pt=presenter.winPt(e,264,240);presenter.setState({cmenu:{...pt,type:'macro-bind',key:id}});};
  if(macro){
    model.insp={icon:'variation',title:macro.name,type:'Macro',ann:'Parameter bindings',rows:[{h:true,label:'Macro'},{p:true,label:'Value',value:macro.value*100,unit:'%',field:{value:macro.value*100,min:0,max:100,step:1,onChange:v=>command('Change '+macro.name,'setMacro',macro.id,v/100)}},
      {p:true,label:'MIDI',value:macro.midi?'CC '+macro.midi.cc+' · channel '+macro.midi.channel:'Unassigned'},
      {b:true,label:'Learn MIDI',icon:'midi',action:()=>presenter.learnMidi(macro.id)},{b:true,label:'Rename…',action:()=>presenter.rename('macro',macro.id,macro.name)},
      {h:true,label:'Destinations'},...macro.bindings.flatMap(binding=>[{b:true,label:binding.destination,action:()=>presenter.goToDestination(binding.destination)},
        ...[0,1].map((index)=>({p:true,label:index?'Maximum':'Minimum',value:binding.range[index]*100,unit:'%',field:{value:binding.range[index]*100,min:0,max:100,step:1,onChange:v=>{const range=[...binding.range];range[index]=v/100;command('Change macro range','bindMacro',macro.id,binding.destination,range);}}}))]),
      {b:true,label:'Add destination…',icon:'plus',action:e=>bindMenu(e,macro.id)}]};
    model.crumbs=[patch.name,'Macros',macro.name].map((label,i)=>({label,sep:i<2,c:'var(--dd-paper-2)'}));
  }
  if(menu?.type==='macro-bind')model.kmenus=[{x:menu.x,y:Math.min(menu.y,model.H-250),title:'Macro destination',items:model.routeDestinations.map(dest=>({label:dest.label,onSelect:()=>{close();command('Bind macro','bindMacro',menu.key,dest.id,[0,1]);}}))}];
  if(menu?.type==='macro'){
    const current=patch.macros.find(m=>m.id===menu.key);if(!current)return;
    model.kmenus=[{x:menu.x,y:Math.max(4,Math.min(menu.y,model.H-300)),title:current.name,items:[{label:'Edit macro',onSelect:()=>{close();model.selectMacro(current.id);}},
      {header:current.bindings.length?'Go to destination':'No destinations'},...current.bindings.map(binding=>({label:binding.destination,onSelect:()=>{close();presenter.goToDestination(binding.destination);}})),
      {separator:true},{label:'Add destination…',onSelect:()=>presenter.setState({cmenu:{...menu,type:'macro-bind'}})},
      {label:'Learn MIDI',onSelect:()=>{close();presenter.learnMidi(current.id);}},{label:'Rename…',onSelect:()=>presenter.rename('macro',current.id,current.name)},
      {separator:true},{label:'Reset to default',shortcut:'Middle-click',onSelect:()=>presenter.resetTarget('macro',current.id)}]}];
  }
}
