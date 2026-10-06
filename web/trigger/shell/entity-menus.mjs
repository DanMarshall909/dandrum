export function entityMenus(presenter,model){
  const menu=presenter.state.cmenu;if(!menu||!['node','src','mod','route','candidate'].includes(menu.type))return;
  const patch=presenter.props.store.getSnapshot().patch,close=()=>presenter.setState({cmenu:null}),action=fn=>()=>{close();fn();},command=(label,method,...args)=>presenter.command(label,method,...args);
  let title=menu.key,items=[];
  if(menu.type==='node'){
    const node=patch.nodes.find(n=>n.id===menu.key),selector=patch.selectors[menu.key];if(selector&&!node){title='Hard';items=[{label:'Open',onSelect:action(()=>{presenter.go('rr');presenter.setState({selection:{kind:'selector',id:menu.key}});})},{label:'Audition',onSelect:action(()=>presenter.auditionNode('snare'))}];}
    else if(node){const root=node.parent===null;title=node.name;items=[{label:'Open',onSelect:action(()=>presenter.openNode(node.id))},{label:'Audition',onSelect:action(()=>presenter.auditionNode(node.id))},
      {label:'Rename…',onSelect:()=>presenter.rename('node',node.id,node.name)},{label:'Duplicate',shortcut:root?'instrument root':'Ctrl+D',disabled:root,onSelect:action(()=>command('Duplicate '+node.name,'duplicateNode',node.id))},
      {separator:true},{label:'Delete',disabled:root,shortcut:root?'instrument root':'Delete',danger:true,onSelect:action(()=>command('Delete '+node.name,'removeNode',node.id))}];}
  }
  if(menu.type==='src'){
    const module=patch.modules.find(m=>'module:'+m.id===menu.key);
    if(module){model.kmenus=[{x:menu.x,y:Math.max(4,Math.min(menu.y,model.H-210)),title:module.type,items:[
      {label:'Preview',onSelect:action(()=>presenter.props.store.auditionSource('module:'+module.id))},
      {label:'Open in Voice',onSelect:action(()=>{presenter.go('voice');presenter.setState({selectedModule:module.id});})},
      {label:'Show in browser',disabled:true,shortcut:'Synth sources live in Voice'},
      {label:'Replace…',onSelect:action(()=>{presenter.go('voice');presenter.setState({dialog:{kind:'voice-template'}});})},
      {label:'Detect transients',disabled:true,shortcut:'Requires an audio asset'},
      {separator:true},{label:'Remove from patch…',onSelect:action(()=>{presenter.go('voice');presenter.setState({dialog:{kind:'voice-template'}});})},
    ]}];return;}
    const region=patch.regions.find(r=>r.id===menu.key||r.name===menu.key),asset=patch.assets.find(a=>a.id===(region?.assetId??menu.key)||a.name===menu.key);if(!asset)return;title=region?.name??asset.name;
    items=[{label:'Preview',disabled:asset.state!=='loaded',shortcut:asset.state!=='loaded'?'asset unavailable':'Click',onSelect:action(()=>presenter.props.store.audition(asset.id,region))},
      {label:'Show in browser',onSelect:action(()=>presenter.setState({browser:true,browserQuery:asset.name}))},
      {label:'Replace…',onSelect:action(()=>presenter.replaceSource?.(asset.id))},{label:'Detect transients',disabled:asset.state!=='loaded',onSelect:action(()=>{presenter.go('slices');presenter.setState({sliceAsset:asset.id});presenter.startAnalysis(asset.id,'transients');})},
      {separator:true},{label:'Remove from patch',danger:true,onSelect:action(()=>command('Remove source','removeAsset',asset.id))}];
  }
  if(menu.type==='mod'){
    const mod=patch.modulators.find(m=>m.id===menu.key||m.name===menu.key);if(!mod)return;title=mod.name;
    const edit=()=>{presenter.go('mod');presenter.setState({pickedMod:mod.id,modInsp:true});};
    items=[{label:'Edit',onSelect:action(edit)},{label:'Add destination…',onSelect:()=>presenter.setState({cmenu:null,dmenu:{x:menu.x,y:menu.y,src:mod.id}})},
      {label:'Show routes',onSelect:action(edit)},{label:'Rename…',onSelect:()=>presenter.rename('modulator',mod.id,mod.name)},
      {separator:true},{label:'Delete modulator',disabled:mod.locked,shortcut:mod.locked?'fixed Amp envelope':'',danger:true,onSelect:action(()=>command('Delete modulator','removeModulator',mod.id))}];
  }
  if(menu.type==='route'){
    const route=patch.routes.find(r=>r.id===menu.key);if(!route)return;title=(patch.modulators.find(m=>m.id===route.source)?.name??route.source)+' → '+route.destination;
    items=presenter.routeMenuItems(route);items.splice(items.length-2,0,{label:'Go to '+route.destination,onSelect:action(()=>presenter.goToDestination(route.destination))});
  }
  if(menu.type==='candidate'){
    const target=model.selectorId,candidate=patch.selectors[target]?.candidates.find(c=>c.id===menu.key);if(!candidate)return;title=candidate.name;
    items=[{label:'Edit',onSelect:action(()=>presenter.setState({candidateSelection:candidate.id}))},{label:candidate.muted?'Unmute':'Mute',onSelect:action(()=>command('Toggle candidate mute','setCandidate',target,candidate.id,{muted:!candidate.muted}))},
      {label:candidate.solo?'Unsolo':'Solo',onSelect:action(()=>command('Toggle candidate solo','setCandidate',target,candidate.id,{solo:!candidate.solo}))},
      {separator:true},{label:'Remove candidate',danger:true,onSelect:action(()=>command('Remove candidate','removeCandidate',target,candidate.id))}];
  }
  model.kmenus=[{x:menu.x,y:Math.max(4,Math.min(menu.y,model.H-items.length*26-44)),title,items}];
}
