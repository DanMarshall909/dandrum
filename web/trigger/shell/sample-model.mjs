import {snapZero} from '../engine/mock-wave.mjs';
export function selectedSample(presenter){
  const patch=presenter.props.store.getSnapshot().patch;
  const rule=patch.rules.find(r=>r.id===presenter.state.zoneSel);
  let source=rule?.source??presenter.state.selectedAsset??'k60f';
  if(patch.selectors[source])source=patch.selectors[source].candidates[0]?.id;
  const region=patch.regions.find(r=>r.id===source)??patch.regions.find(r=>r.id==='k60f')??patch.regions[0];
  const asset=patch.assets.find(a=>a.id===(region?.assetId??source))??patch.assets.find(a=>a.state==='loading');
  return {patch,rule,region:region??(asset?{id:asset.id,assetId:asset.id,start:0,end:1,root:60,fades:{in:0,out:0},loop:null,playback:'once'}:null),asset};
}

export function sampleModel(presenter,model){
  const store=presenter.props.store,{patch,rule,region:accepted,asset}=selectedSample(presenter);
  if(!accepted||!asset)return;
  if(!presenter.regionDefaults.has(accepted.id))presenter.regionDefaults.set(accepted.id,structuredClone(accepted));
  const defaults=presenter.regionDefaults.get(accepted.id);
  const draft=presenter.state.sampleDraft;
  const region=draft?.id===accepted.id?{...accepted,...draft}:accepted;
  const peaks=presenter.state.wavePeaks?.[asset.id]??[],normal=patch.history.find(op=>op.regionId===accepted.id&&op.name==='Normalize'&&op.on),gain=normal?Math.pow(10,(normal.config.target+1.2)/20):1;
  model.pianoPeaks=peaks.map(v=>Math.min(1,v*gain));model.partialPeaks=model.pianoPeaks.map((v,i)=>i/Math.max(1,peaks.length)<(asset.progress??0)?v:0);
  model.seamA=peaks.slice(Math.max(0,Math.round((region.loop?.end??1)*peaks.length)-40),Math.round((region.loop?.end??1)*peaks.length));model.seamB=peaks.slice(Math.round((region.loop?.start??0)*peaks.length),Math.round((region.loop?.start??0)*peaks.length)+40);
  model.loadingProgress=asset.progress??0;model.isSample=model.page==='sample'&&!model.isDrop;model.isEmpty=false;
  model.snapPosition=presenter.state.snapZero===false?null:value=>snapZero(asset.id,value);
  model.viewport=presenter.state.waveViewport??{zoom:1,offset:0};model.setViewport=viewport=>presenter.setState({waveViewport:viewport});
  const duration=asset.duration,seconds=v=>(v*duration).toFixed(3)+' s';
  const edit=(label,delta)=>store.operation(label,'setRegion',region.id,delta).catch(error=>presenter.setState({modNote:error.message}));
  const field=(label,read,delta,format)=>({label,key:'Region '+label,v:read(region),default:read(defaults),t:format(read(region)),
    set:value=>presenter.setState({sampleDraft:{id:region.id,...delta(value)}}),
    commit:value=>{const revision=++presenter.sampleRevision;return edit('Change '+label,delta(value)).finally(()=>{
      if(presenter.sampleRevision===revision)presenter.setState({sampleDraft:null});});},
    parse:text=>{const n=Number.parseFloat(text.replace('−','-'));return Number.isFinite(n)?n/(label.startsWith('Fade')?duration*1000:duration):null;},mods:[],bi:false});
  model.sampleKnobGroups[0].knobs=[
    field('Start',r=>r.start,v=>({start:Math.min(v,region.end-.0001)}),seconds),
    field('End',r=>r.end,v=>({end:Math.max(v,region.start+.0001)}),seconds),
    field('Fade in',r=>r.fades?.in??0,v=>({fades:{...region.fades,in:Math.min(v,region.end-region.start)}}),v=>Math.round(v*duration*1000)+' ms'),
    field('Fade out',r=>r.fades?.out??0,v=>({fades:{...region.fades,out:Math.min(v,region.end-region.start)}}),v=>Math.round(v*duration*1000)+' ms'),
  ];
  Object.assign(model,{assetId:asset.id,regionId:region.id,assetName:asset.name,waveLabel:asset.name+' · '+duration.toFixed(3)+' s',
    playMode:region.playback,reverse:!!region.reverse,loop:!!region.loop,loopS:region.loop?.start,loopE:region.loop?.end,xfade:region.loop?.crossfade,
    hRs:region.start,hRe:region.end,hFi:region.fades?.in??0,hFo:region.fades?.out??0,
    root:region.root??rule?.root??60,setRoot:value=>edit('Change root',{root:Math.round(value)}),
    onWaveClick:()=>store.audition(asset.id,region),setRegion:delta=>edit('Edit region',delta),
    setPlayback:playback=>edit('Change playback',{playback,loop:['loop','pp','sus'].includes(playback)?region.loop??{start:.55,end:.82,crossfade:.025}:null}),
    toggleReverse:()=>edit('Reverse sample',{reverse:!region.reverse}),
    setLoop:(field,value)=>edit('Change loop '+field,{loop:{...region.loop,[field]:value}}),
    snap:presenter.state.snapZero??true,setSnap:value=>presenter.setState({snapZero:value}),
    duration,missing:asset.state==='missing',unsup:asset.state==='unsupported',isLoading:asset.state==='loading',waveReady:asset.state!=='loading',
    assetChoices:patch.assets.filter(a=>a.state==='loaded').map(a=>({id:a.id,label:a.name})),
    selectAsset:id=>store.operation('Replace sample','replaceRegionAsset',region.id,id).catch(error=>presenter.setState({modNote:error.message})),
  });
  model.detectedPitch=asset.analysis?.note!=null?presenter.nn(asset.analysis.note)+' +'+(asset.analysis.cents??0)+'¢':null;
  model.banner=model.missing||model.unsup;model.missingMessage=asset.name+' was not found. Locate the source to restore every region that uses it.';
  model.meta=[{k:'Format',v:`${asset.sampleRate/1000} kHz · ${asset.bits}-bit · ${asset.channels===2?'stereo':'mono'}`},
    {k:'Length',v:duration.toFixed(3)+' s'},{k:'Peak',v:asset.analysis?.peak!=null?asset.analysis.peak+' dBFS':'Not analysed'},{k:'Loudness',v:asset.analysis?.lufs!=null?asset.analysis.lufs+' LUFS':'Not analysed'},
    {k:'Pitch',v:asset.analysis?.note!=null?presenter.nn(asset.analysis.note)+' '+(asset.analysis.cents??0)+'¢':'Not analysed'},{k:'Source',v:asset.state==='loaded'?'external · cached':asset.state}];
}
