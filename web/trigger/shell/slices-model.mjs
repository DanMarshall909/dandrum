import {latestAnalysis} from './analysis-model.mjs';
const p=(label,value,field,unit)=>({p:true,label,value,field,unit}),h=label=>({h:true,label});
export function slicesModel(presenter,model){
  const {patch}=presenter.props.store.getSnapshot(),asset=patch.assets.find(a=>a.id===presenter.state.sliceAsset)??patch.assets.find(a=>a.id===(patch.regions.find(r=>r.id==='amen')?.assetId??'amen'))??patch.assets[0];
  if(!asset)return;
  const regions=patch.regions.filter(r=>r.assetId===asset.id&&r.kind==='slice').sort((a,b)=>a.start-b.start);
  const ids=(presenter.state.sliceSelection??[regions[2]?.id??regions[0]?.id]).filter(id=>regions.some(r=>r.id===id)),selected=regions.filter(r=>ids.includes(r.id));
  const command=(label,method,...args)=>presenter.command(label,method,...args),job=latestAnalysis(presenter,asset.id,'transients'),sensitivity=presenter.state.sensitivity??.62;
  const ticks=job?.state==='done'?job.result.map((pos,i)=>({pos,strength:[1,.3,.7,.35,.95,.6,.4,.25,.9,.3,.65,.45,.95,.32,.7][i%15]})):[];
  const markers=ticks.filter(t=>t.strength>=1-sensitivity).map(t=>t.pos),from=presenter.state.sliceMapFrom??regions[0]?.note??24;
  const select=(r,event={})=>{let next=[r.id];if(event.ctrlKey||event.metaKey)next=ids.includes(r.id)?ids.filter(id=>id!==r.id):[...ids,r.id];
    else if(event.shiftKey&&ids.length){const a=regions.findIndex(r=>r.id===ids[0]),b=regions.indexOf(r);next=regions.slice(Math.min(a,b),Math.max(a,b)+1).map(r=>r.id);}
    presenter.setState({sliceSelection:next,histSel:null});presenter.props.store.audition(asset.id,r);};
  const apply=()=>command('Apply transient slices','setSlices',asset.id,markers,{startNote:from});
  const divide=(count=presenter.state.sliceCount??8)=>{const grid=presenter.state.sliceDivision==='grid',step=60/(asset.tempo??172)/4/asset.duration,markers=grid?Array.from({length:count},(_,i)=>Math.min(.999,Math.round(i/count/step)*step)):Array.from({length:count},(_,i)=>i/count);return command('Divide slices','setSlices',asset.id,markers,{startNote:from,method:grid?'grid':'even'});};
  const cursor=presenter.state.sliceCursor??.29,add=()=>{const r=regions.find(r=>cursor>r.start&&cursor<r.end);if(r)return command('Split slice','splitSlice',r.id,cursor);};
  const remove=()=>ids.length&&command('Delete slices','removeSlices',ids);
  model.slicePeaks=presenter.state.wavePeaks?.[asset.id]??[];
  model.sliceAssetId=asset.id;model.sliceOwnerId=patch.regions.find(r=>r.kind==='sample'&&r.assetId===asset.id)?.id??asset.id;model.sliceAssetName=asset.name;model.sliceDuration=asset.duration;model.sliceRegionId=selected[0]?.id;
  model.slices=regions.map(r=>({id:r.id,pos:r.start,name:r.name}));model.sliceCount=presenter.state.sliceCount??8;model.sliceTotal=regions.length;
  model.sliceSelectionCount=selected.length;model.selSlice=Math.max(0,regions.findIndex(r=>r.id===ids.at(-1)));model.sliceCursor=cursor;
  Object.assign(model,{sliceMapFrom:from,setSliceMapFrom:v=>presenter.setState({sliceMapFrom:Math.round(v)}),mapSlices:()=>command('Map slices sequentially','mapSlices',asset.id,from),
    sliceDivision:presenter.state.sliceDivision??'even',setSliceDivision:value=>presenter.setState({sliceDivision:value}),divideSlices:divide,
    setSliceCount:v=>{const count=Math.round(v);presenter.setState({sliceCount:count});divide(count);},addSliceMarker:add,deleteSliceMarkers:remove,
    clearSliceMarkers:()=>command('Clear markers','setSlices',asset.id,[0],{startNote:from}),moveSliceMarker:(id,value)=>command('Move slice marker','moveSliceBoundary',id,value),
    stepSlice:direction=>{const at=regions.findIndex(r=>r.id===ids.at(-1));if(regions.length)select(regions[Math.max(0,Math.min(regions.length-1,at+direction))]);},
    detectTransients:()=>presenter.startAnalysis(asset.id,'transients'),trRun:job?.state==='running',trDone:job?.state==='done'&&presenter.state.dismissedTransients!==job.id,trFail:job?.state==='failed',trShown:!!job,
    analysisProgress:job?.progress??0,analysisError:job?.message,sensitivity,setSensitivity:value=>presenter.setState({sensitivity:value}),transientTotal:ticks.length,applySliceTotal:new Set([0,...markers]).size,
    applyTransients:apply,discardTransients:()=>presenter.setState({dismissedTransients:job.id}),cancelTransients:()=>presenter.props.store.cancelJob(job.id),
    transients:ticks.map(t=>({x:t.pos*100,h:Math.round(6+t.strength*18),c:t.strength>=1-sensitivity?'var(--dd-paper-1)':'var(--dd-paper-4)'})),
    onSliceWaveClick:event=>{const box=event.currentTarget.getBoundingClientRect(),pos=Math.max(0,Math.min(1,(event.clientX-box.left)/box.width));presenter.setState({sliceCursor:pos});const r=regions.find(r=>pos>=r.start&&pos<r.end);if(r)select(r,event);},
  });
  model.sliceRows=regions.map((r,i)=>({id:r.id,n:i+1,name:r.name,note:(r.note??24+i)+' '+presenter.nn(r.note??24+i),range:(r.start*asset.duration).toFixed(3)+'–'+(r.end*asset.duration).toFixed(3),
    tune:(r.tune??0)+' st',gain:(r.gain??0).toFixed(1)+' dB',pan:r.pan?Math.round(Math.abs(r.pan)*100)+(r.pan<0?' L':' R'):'C',mode:r.playback==='gated'?'Gated':'Once',choke:r.choke||'—',out:patch.buses.find(b=>b.id===r.output)?.name??'Inherit · Main',
    bg:ids.includes(r.id)?'var(--dd-vermilion-wash)':'transparent',edge:ids.includes(r.id)?'inset 2px 0 0 var(--dd-vermilion)':'none',play:e=>select(r,e),
    drag:e=>{presenter.setState({dragSrc:{id:r.id,assetId:asset.id,name:r.name,kind:'slice'}});e.dataTransfer.setData('text/plain',r.id);}}));
  model.tabs=model.tabs.map(tab=>tab.id==='slices'?{...tab,badge:String(regions.length)}:tab);
  if(model.page!=='slices')return;
  presenter.sliceKeyboard={add,remove,step:model.stepSlice};
  const mixed=key=>selected.every(r=>(r[key]??0)===(selected[0]?.[key]??0))?selected[0]?.[key]??0:'Mixed';
  const numeric=(label,key,min,max,unit)=>p(label,mixed(key),{value:mixed(key)==='Mixed'?0:mixed(key),min,max,step:key==='pan'?.01:1,onChange:value=>command('Change slice '+label,'editSlices',ids,{[key]:value})},unit);
  model.insp={icon:'slice',title:selected.length+' slice'+(selected.length===1?'':'s'),type:'Slices',ann:'Asset Region + Trigger Rule',rows:[h('Slices'),p('Notes',selected.map(r=>r.note+' '+presenter.nn(r.note)).join(', ')),
    p('Play',mixed('playback'),{value:mixed('playback'),options:model.playOpts,onChange:value=>command('Change slice playback','editSlices',ids,{playback:value})}),
    numeric('Tune','tune',-48,48,'st'),numeric('Gain','gain',-60,12,'dB'),numeric('Pan','pan',-1,1),numeric('Choke group','choke',0,127),
    p('Output',mixed('output'),{value:mixed('output')||'inherit',options:[{id:'inherit',label:'Inherit'},...patch.buses.map(b=>({id:b.id,label:b.name}))],onChange:value=>command('Route slices','editSlices',ids,{output:value})}),
    {t:true,label:'Mixed values are changed together for every selected slice.'}]};
  model.crumbs=[patch.name,asset.name,'Slices'].map((label,i)=>({label,sep:i<2,c:'var(--dd-paper-2)'}));
  const menu=presenter.state.cmenu;if(menu?.type==='slice'){
    const r=regions.find(r=>r.id===menu.key),next=regions[regions.indexOf(r)+1];if(!r)return;
    const close=fn=>()=>{presenter.setState({cmenu:null});fn();};
    model.kmenus=[{x:menu.x,y:menu.y,title:r.name,items:[{label:'Preview',onSelect:close(()=>select(r))},
      {label:'Map to key…',onSelect:close(()=>presenter.setState({sliceSelection:[r.id],dialog:{kind:'slice-note',id:r.id,value:r.note??24}}))},
      {label:'Split at cursor',disabled:cursor<=r.start||cursor>=r.end,onSelect:close(()=>command('Split slice','splitSlice',r.id,cursor))},
      {label:'Merge with next',disabled:!next||Math.abs(r.end-next.start)>.0001,onSelect:close(()=>command('Merge slices','mergeSlices',r.id,next.id))},
      {separator:true},{label:'Delete slice',danger:true,onSelect:close(()=>command('Delete slice','removeSlice',r.id))}]}];
  }
}
