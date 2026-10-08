const property=(label,value,field)=>({p:true,label,value,field});
export function mappingModel(presenter,model){
  const patch=presenter.props.store.getSnapshot().patch,zones=model.zones;
  const selected=zones.find(z=>z.id===presenter.state.zoneSel)??zones.find(z=>z.id==='k60f')??zones[0],group=selected?.g??presenter.state.selection?.id??patch.nodes.find(n=>n.kind==='group')?.id;
  const current=zones.filter(z=>z.g===group),command=(label,method,...args)=>presenter.command(label,method,...args);
  model.mappingSummary=zones.length+' zones · '+new Set(zones.map(z=>z.g)).size+' groups';model.mappingTitle=patch.name;
  const duplicate=()=>{if(!selected)return;presenter.copyRange(selected);return presenter.paste(selected.lo,true);};
  model.duplicateZone=duplicate;model.deleteZone=()=>selected&&presenter.del(selected,false);model.noZone=!selected;
  model.mapByRoot=()=>{
    const bands=Map.groupBy?Map.groupBy(current,z=>z.velLo+'-'+z.velHi):new Map();
    if(!Map.groupBy)for(const z of current){const key=z.velLo+'-'+z.velHi;if(!bands.has(key))bands.set(key,[]);bands.get(key).push(z);}
    const replacements=new Map();for(const list of bands.values()){
      const sorted=[...list].sort((a,b)=>a.root-b.root),low=Math.min(...list.map(z=>z.lo)),high=Math.max(...list.map(z=>z.hi));
      sorted.forEach((z,i)=>replacements.set(z.id,{...z,lo:i?Math.floor((sorted[i-1].root+z.root)/2)+1:low,hi:i+1<sorted.length?Math.floor((z.root+sorted[i+1].root)/2):high}));
    }return presenter.commit(zones.map(z=>replacements.get(z.id)??z),'Map by root');
  };
  model.mapSequential=()=>{const start=Math.min(...current.map(z=>z.root??z.lo)),rows=current.slice().sort((a,b)=>a.root-b.root||a.velLo-b.velLo),offset=new Map(rows.map((z,i)=>[z.id,Math.min(127,start+i)]));
    return presenter.commit(zones.map(z=>offset.has(z.id)?{...z,lo:offset.get(z.id),hi:offset.get(z.id),root:offset.get(z.id)}:z),'Map sequentially');};
  model.mapVelocity=async()=>{
    if(!selected?.sourceId)return;const target=selected.target??group;
    if(!patch.selectors[target])await command('Create velocity layers','assignSource',target,selected.sourceId,'velocity');
    await command('Use velocity layers','setSelector',target,{policy:'velocity'});presenter.go('vel');presenter.setState({selectorTarget:target});
  };
  model.onKeyOn=(note,velocity)=>presenter.props.store.noteOn(note,velocity);model.onKeyOff=note=>presenter.props.store.noteOff(note);
  model.onMapDrop=e=>{
    const point=presenter.pointNote(e);if(!point)return;const hits=zones.filter(z=>point.note>=z.lo&&point.note<=z.hi&&(point.vel==null||point.vel>=z.velLo&&point.vel<=z.velHi)),hit=hits.at(-1);
    if(e.dataTransfer.files.length){e.preventDefault();e.stopPropagation();presenter.queueImport(e.dataTransfer.files,hit&&!e.shiftKey&&hit.source==='Sample'?{replace:hit.sourceId}:{group:hit?.g??group,startNote:point.note});return;}
    const source=presenter.state.dragSrc;if(!source)return;e.preventDefault();e.stopPropagation();
    if(hit&&!e.shiftKey)command('Replace zone source','updateRule',hit.id,{source:source.id});
    else presenter.commit([...zones,{id:presenter.uid(),g:hit?.g??group??'root',name:source.name,sourceId:source.id,lo:hit?.lo??point.note,hi:hit?.hi??point.note,root:point.note,velLo:hit?.velLo??1,velHi:hit?.velHi??127}],'Add source zone');
    presenter.setState({dragSrc:null});
  };
  presenter.mappingKeyboard={duplicate,step:direction=>{if(!zones.length)return;const at=zones.indexOf(selected);presenter.setState({zoneSel:zones[(at+direction+zones.length)%zones.length].id});}};
  if(model.page!=='mapping')return;
  if(!selected){model.insp={icon:'keyboard',title:'Key map',type:'Mapping',rows:[{t:true,label:'No zones. Drop samples on the key map or import files to create them.'}]};return;}
  const numeric=(label,key,min,max)=>property(label,selected[key],{value:selected[key],min,max,step:1,onChange:value=>command('Change zone '+label,'updateRule',selected.id,{[{lo:'noteLo',hi:'noteHi'}[key]??key]:value})});
  const related=presenter.relations(zones).filter(r=>r.a.id===selected.id||r.b.id===selected.id),region=patch.regions.find(r=>r.id===selected.sourceId),asset=patch.assets.find(a=>a.id===(region?.assetId??selected.sourceId));
  const overlaps=zones.filter(z=>z.id!==selected.id&&z.g===selected.g&&z.lo<=selected.hi&&z.hi>=selected.lo&&z.velLo<=selected.velHi&&z.velHi>=selected.velLo);
  model.insp={icon:'keyboard',title:selected.name,type:'Zone',rows:[{h:true,label:'Keys'},numeric('Low','lo',0,selected.hi),numeric('High','hi',selected.lo,127),numeric('Root','root',0,127),{h:true,label:'Velocity'},numeric('Low','velLo',1,selected.velHi),numeric('High','velHi',selected.velLo,127),
    {h:true,label:'Neighbours'},property('Key crossfade',related.filter(r=>r.kind==='xf').map(r=>presenter.rangeTxt(r.lo,r.hi)).join(', ')||'None'),property('Gap',related.filter(r=>r.kind==='gap').map(r=>presenter.rangeTxt(r.lo,r.hi)).join(', ')||'None'),property('Edge drag','Pushes neighbour'),
    {h:true,label:'Sample'},property('File',asset?.name??selected.source),property('Overlaps',overlaps.map(z=>z.name).join(', ')||'None'),{b:true,label:'Replace sample…',icon:'folder',disabled:!region,action:()=>presenter.replaceRegion?.(region.id)}]};
  model.crumbs=[patch.name,patch.nodes.find(n=>n.id===group)?.name??'Mapping',selected.name].map((label,i)=>({label,sep:i<2,c:'var(--dd-paper-2)'}));
}
