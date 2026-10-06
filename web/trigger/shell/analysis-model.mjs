export const analysisActions={
  startAnalysis(assetId,kind){
    try{const job=this.props.store.analyse(assetId,kind);this.setState({analysisJob:job.id,modNote:null});return job;}
    catch(error){this.setState({modNote:error.message});}
  },
};
export function latestAnalysis(presenter,assetId,kind){return Object.values(presenter.props.store.getSnapshot().jobs).filter(j=>j.assetId===assetId&&(!kind||j.kind===kind)).at(-1);}
export function analysisModel(presenter,model){
  const job=latestAnalysis(presenter,model.assetId),pitch=latestAnalysis(presenter,model.assetId,'pitch'),loudness=latestAnalysis(presenter,model.assetId,'loudness'),loops=latestAnalysis(presenter,model.assetId,'loops');
  model.analysis=job?{...job,cancel:()=>presenter.props.store.cancelJob(job.id),discard:()=>presenter.setState({dismissedAnalysis:job.id})}:null;
  if(presenter.state.dismissedAnalysis===job?.id)model.analysis=null;
  model.loopSuggestions=loops?.state==='done'?loops.result.map(loop=>({label:(loop.start*model.duration).toFixed(3)+'–'+(loop.end*model.duration).toFixed(3)+' s',
    selected:model.loopS===loop.start&&model.loopE===loop.end,select:()=>presenter.command('Apply suggested loop','setRegion',model.regionId,{loop,playback:'sus'})})):[];
  model.findLoops=()=>presenter.startAnalysis(model.assetId,'loops');
  if(pitch?.state==='done'){model.detectedPitch=presenter.nn(pitch.result.note)+' '+pitch.result.cents+'¢';model.meta=model.meta.map(row=>row.k==='Pitch'?{...row,v:model.detectedPitch}:row);}
  if(loudness?.state==='done')model.meta=model.meta.map(row=>row.k==='Loudness'?{...row,v:loudness.result.lufs+' LUFS'}:row.k==='Peak'?{...row,v:loudness.result.peak+' dBFS'}:row);
  if(job?.state==='done')model.analysis.apply=job.kind==='pitch'?()=>presenter.command('Apply detected root','setRegion',model.regionId,{root:job.result.note}):job.kind==='loops'?model.loopSuggestions[0]?.select:null;
}
