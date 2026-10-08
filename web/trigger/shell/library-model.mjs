export function libraryModel(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,state=presenter.state,children=id=>patch.nodes.filter(node=>node.parent===id);
  const selected=state.selection?.id??({sample:'k60',slices:'break',mapping:'keys',layers:state.st==='rr'?'snare-hard':'snare',voice:'keys',mod:'keys',routing:'root',fx:'drums'}[model.page]);
  const tree=[];
  const add=(node,depth)=>{
    const rules=patch.rules.filter(rule=>rule.group===node.id||rule.target===node.id);
    const route=node.kind==='instrument'?'routing':node.id==='break'?'slices':node.id==='drums'?'fx':node.kind==='group'?'mapping':node.id==='snare'?'vel':'sample';
    const go=()=>{presenter.go(route);presenter.setState({selection:{kind:'node',id:node.id},zoneSel:rules.find(r=>r.id.endsWith('f'))?.id??rules[0]?.id??state.zoneSel});};
    const eligible=presenter.state.dragSrc&&presenter.dropInfo(node.id),hover=eligible&&presenter.state.nodeOver===node.id;
    tree.push({id:node.id,label:node.name,detail:node.kind==='instrument'?children(node.id).length+' groups':node.kind==='group'?node.id==='drums'?children(node.id).length+' sounds':rules.length+(node.id==='break'?' slices':' zones'):node.note!=null?node.note+' '+presenter.nn(node.note):node.root!=null?'p · f':'sound',
      pad:4+depth*14,icon:node.kind==='instrument'?'layers':node.kind==='group'?node.id==='break'?'slice':'keyboard':'level',sel:node.id===selected,active:node.id==='k60'&&model.page==='sample',dim:false,go,
      drop:eligible?'inset 0 0 0 1px '+(hover?'var(--dd-vermilion)':'var(--dd-line-3)'):'none',dbg:hover?'var(--dd-vermilion-wash)':'transparent'});
    children(node.id).forEach(child=>add(child,depth+1));
    if(node.id==='snare'&&patch.selectors['snare-hard'])tree.push({id:'snare-hard',label:'Hard',detail:'×'+patch.selectors['snare-hard'].candidates.length,pad:4+(depth+1)*14,icon:'alternate',sel:selected==='snare-hard',drop:'none',dbg:'transparent',
      go:()=>{presenter.go('rr');presenter.setState({selection:{kind:'selector',id:'snare-hard'},zoneSel:'snH'});}});
  };
  patch.nodes.filter(node=>node.parent===null).forEach(root=>add(root,0));model.tree=tree;
  const makeItem=(asset,region)=>({id:region?.id??asset.id,assetId:asset.id,loadProgress:asset.state==='loading'?asset.progress:null,name:region?.name??asset.name,detail:region?'slice':asset.state==='loaded'?asset.duration.toFixed(2)+' s':asset.state,
    icon:asset.state==='missing'?'file-missing':region?'slice':'level',ic:asset.state==='missing'?'var(--dd-error)':'var(--dd-paper-3)',playing:false,pct:0,
    bg:asset.id===model.assetId?'var(--dd-vermilion-wash)':'transparent',edge:asset.id===model.assetId?'inset 2px 0 0 var(--dd-vermilion)':'none',
    play:()=>presenter.play(region?.id??asset.id),drag:e=>{e.dataTransfer.setData('text/plain',region?.id??asset.id);e.dataTransfer.effectAllowed='copy';presenter.setState({dragSrc:{name:region?.name??asset.name,id:region?.id??asset.id,kind:region?'slice':'sample'}});}});
  const groups=[...new Set(patch.assets.map(asset=>asset.group??'imported'))].sort((a,b)=>['keys','drums','break'].indexOf(a)-['keys','drums','break'].indexOf(b));
  model.srcGroups=groups.map(id=>{
    const assets=patch.assets.filter(asset=>(asset.group??'imported')===id),open=!!state.openSrc?.[id];
    const items=assets.flatMap(asset=>{const slices=patch.regions.filter(region=>region.assetId===asset.id&&region.kind==='slice');return slices.length?slices.map(region=>makeItem(asset,region)):[makeItem(asset)];});
    return {id,name:{keys:'Felt Piano',drums:'Drum samples',break:'amen_172.wav'}[id]??patch.nodes.find(n=>n.id===id)?.name??'Imported',open,chev:open?'chevron-down':'chevron-right',summary:items.length,hbg:'transparent',items,
      toggle:()=>presenter.setState({openSrc:{...presenter.state.openSrc,[id]:!open}})};
  });
  const patches=patch.modules.filter(m=>m.type==='Lush');if(patches.length){const open=!!state.openSrc?.patch;
    model.srcGroups.push({id:'patch',name:'Dandrum patches',open,chev:open?'chevron-down':'chevron-right',summary:patches.length,hbg:'transparent',toggle:()=>presenter.setState({openSrc:{...presenter.state.openSrc,patch:!open}}),items:patches.map(m=>({id:'module:'+m.id,name:m.type,detail:'synth',icon:'variation',ic:'var(--dd-paper-3)',bg:'transparent',edge:'none',playing:false,pct:0,play:()=>presenter.props.store.auditionSource('module:'+m.id),drag:e=>{e.dataTransfer.setData('text/plain','module:'+m.id);presenter.setState({dragSrc:{id:'module:'+m.id,name:m.type,kind:'patch'}});}}))});}
  const query=(state.browserQuery??'').toLowerCase(),filter=state.browserAssetFilter??'all';
  model.browserQuery=state.browserQuery??'';model.setBrowserQuery=value=>presenter.setState({browserQuery:value});
  model.browserAssetFilter=filter;model.setBrowserAssetFilter=value=>presenter.setState({browserAssetFilter:value});
  model.assets=patch.assets.filter(asset=>asset.name.toLowerCase().includes(query)&&(filter!=='ir'||asset.group==='ir')&&(filter!=='fav'||asset.favourite)).map(asset=>({
    ...makeItem(asset),id:asset.id,row:true,label:asset.name,icon:asset.state==='missing'?'file-missing':'level',dur:asset.duration.toFixed(2)+' s',sel:asset.id===model.assetId,play:()=>presenter.play(asset.id)}));
  model.openRail=kind=>presenter.setState({rail:presenter.state.rail===kind?null:kind});model.rail=presenter.state.rail;
  if(model.isMin)model.toggleBrowser=()=>presenter.setState({rail:null});
  model.addGroup=()=>presenter.command('Add group','addNode','root','group');
  model.onNodeDrop=e=>{
    const target=e.target.closest('[data-node]')?.dataset.node;if(!target)return;
    const node=patch.nodes.find(n=>n.id===target),source=presenter.state.dragSrc;
    if(e.dataTransfer.files.length){e.preventDefault();e.stopPropagation();presenter.queueImport(e.dataTransfer.files,node?.kind==='group'?{group:target}:{target});return;}
    if(!source||!presenter.dropInfo(target))return;e.preventDefault();e.stopPropagation();
    presenter.command('Assign '+source.name,'assignSource',target,source.id);presenter.setState({dragSrc:null,nodeOver:null});
  };
  model.onNodeOver=e=>{const target=e.target.closest('[data-node]')?.dataset.node;
    if(target&&presenter.dropInfo(target)&&(presenter.state.dragSrc||Array.from(e.dataTransfer.types).includes('Files'))){e.preventDefault();e.dataTransfer.dropEffect='copy';presenter.setState({nodeOver:target});}};
  if(model.isLoading)model.status={kind:'busy',title:'Loading',text:model.assetName+' · '+Math.round(model.loadingProgress*100)+'%'};
  if(!state.importing&&!state.lastImportError&&!model.isLoading&&!model.missing&&!model.unsup&&!model.isEmpty&&!model.isDrop)
    model.status={kind:'ok',title:'Ready',text:model.isMin?'':`${patch.rules.length} zones · ${patch.assets.length} samples`};
}
