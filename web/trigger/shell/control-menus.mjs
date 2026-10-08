import {destinationFor} from './modulation-model.mjs';
export const controlActions={
  goToDestination(destination){
    const key=Object.entries({Cutoff:'Filter cutoff',Reso:'Filter resonance',Drive:'Filter drive',Tune:'Pitch',Start:'Sample start',Level:'Amp gain'}).find(([,value])=>value===destination)?.[0]??destination;
    this.go(destination.startsWith('Send')?'routing':destination.startsWith('Sample')?'sample':'voice');
    this.setState({flashKey:key,modNote:'Showing '+destination});clearTimeout(this.flashT);this.flashT=setTimeout(()=>this.setState({flashKey:null}),1800);
  },
  learnMidi(target){this.command('Learn MIDI','learnMidi',target).then(()=>this.setState({dialog:{kind:'midi-learn',target}}));},
};
export function controlMenus(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,menu=presenter.state.kmenu,dm=presenter.state.dmenu;
  const close=()=>presenter.setState({kmenu:null,cmenu:null,dmenu:null});
  const routeItems=route=>[
    {label:'Edit',onSelect:()=>{presenter.go('mod');presenter.setState({selectedRoute:route.id,kmenu:null,cmenu:null});}},
    {label:'Invert',disabled:route.locked,shortcut:route.locked?'fixed route':'',onSelect:()=>{close();presenter.command('Invert route','updateRoute',route.id,{amount:-route.amount});}},
    {label:route.on?'Bypass':'Enable',disabled:route.locked,shortcut:route.locked?'fixed route':'',onSelect:()=>{close();presenter.command('Toggle route','updateRoute',route.id,{on:!route.on});}},
    {separator:true},{label:'Remove',danger:true,disabled:route.locked,shortcut:route.locked?'fixed Amp envelope':'Delete',onSelect:()=>{close();presenter.command('Remove route','removeRoute',route.id);}},
  ];
  presenter.routeMenuItems=routeItems;
  const fit=(x,y,title,items)=>({x:Math.max(4,Math.min(x,model.W-264)),y:Math.max(4,Math.min(y,model.H-Math.min(items.length*26+44,model.H-8))),title,items});
  if(dm){
    model.kmenus=[fit(dm.x,dm.y,'Add destination',model.routeDestinations.map(dest=>({label:dest.label,onSelect:()=>{close();presenter.command('Add route','addRoute',{source:dm.src,destination:dest.id,amount:.25,polarity:'Uni',curve:'Linear',on:true});}})))];return;
  }
  if(!menu)return;
  const destination=destinationFor(menu.label),routes=patch.routes.filter(r=>r.destination===destination),path=menu.path??[],setPath=path=>presenter.setState({kmenu:{...menu,path}});
  const midiTarget=patch.params[menu.label]?.id;
  model.kmenus=[fit(menu.x,menu.y,menu.label,[
    {label:'Modulation',submenu:true,icon:'modulate',onSelect:()=>setPath(['mod'])},
    {label:'Learn MIDI',disabled:!midiTarget,shortcut:!midiTarget?'Not a live parameter':'',onSelect:()=>{close();presenter.learnMidi(midiTarget);}},
    {label:'Map to macro…',submenu:true,onSelect:()=>setPath(['macro'])},{separator:true},
    {label:'Reset to default',disabled:!patch.params[menu.label]&&!presenter.controlKnobs?.some(k=>k.key===menu.label),shortcut:'Middle-click',onSelect:()=>presenter.resetTarget('knob',menu.label)},
  ])];
  const direction=menu.x+3*264>model.W?-1:1;
  if(path[0]==='macro')model.kmenus.push(fit(menu.x+direction*264,menu.y+30,'Map '+menu.label,patch.macros.map(m=>({label:m.name,onSelect:()=>{close();presenter.command('Bind macro','bindMacro',m.id,destination,[0,1]);}}))));
  if(path[0]!=='mod')return;
  const items=[{label:'Add',submenu:true,icon:'plus',onSelect:()=>setPath(['mod','add'])},...(routes.length?[{separator:true},{header:'On '+menu.label}]:[]),
    ...routes.map(r=>({label:(patch.modulators.find(m=>m.id===r.source)?.name??r.source)+' '+Math.round(r.amount*100)+'%',submenu:true,onSelect:()=>setPath(['mod',r.id])}))];
  model.kmenus.push(fit(menu.x+direction*264,menu.y+30,'Modulation',items));
  if(path[1]==='add')model.kmenus.push(fit(menu.x+direction*528,menu.y+60,'Add modulation · '+menu.label,patch.modulators.map(m=>({label:m.name,disabled:routes.some(r=>r.source===m.id),shortcut:routes.some(r=>r.source===m.id)?'added':'',onSelect:()=>{close();presenter.command('Add route','addRoute',{source:m.id,destination,amount:.25,polarity:'Uni',curve:'Linear',on:true});}}))));
  else if(path[1]){const route=routes.find(r=>r.id===path[1]);if(route)model.kmenus.push(fit(menu.x+direction*528,menu.y+60,(patch.modulators.find(m=>m.id===route.source)?.name??route.source)+' → '+menu.label,routeItems(route)));}
  const depth=model.kmenus.length-1,dir=menu.x>model.W/2?-1:1,root=dir>0?Math.max(4,Math.min(menu.x,model.W-264*(depth+1)-4)):Math.max(264*depth+4,Math.min(menu.x,model.W-264-4));
  model.kmenus.forEach((item,i)=>item.x=root+i*264*dir);
}
