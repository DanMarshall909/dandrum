import {clone,entity} from './validation.mjs';
export class AssetsJobs {
  constructor(engine,{loadMs=600,analysisMs=900}={}){this.engine=engine;this.loadMs=loadMs;this.analysisMs=analysisMs;this.jobs=new Map();}
  async importAssets(files){
    const e=this.engine;e.call('importAssets',[files]);const epoch=e.epoch;const assets=[];
    for(const file of files){
      const id=e.id('asset'),name=file.name??file.uri?.split('/').at(-1)??'sample.wav';
      const asset={id,name,uri:file.uri??`mock://${name}`,state:'loading',progress:0,duration:file.duration??4.82,sampleRate:48000,bits:24,channels:2,cached:false,group:'imported'};
      e.patch.assets.push(asset);e.emit('asset',clone(asset));
      for(const progress of [.25,.62,1]){await e.wait(this.loadMs/3);if(e.epoch!==epoch)throw new Error('Patch replaced during import');asset.progress=progress;e.emit('asset',clone(asset));}
      asset.state=e.switches.unsupportedNext||! /\.(wav|aiff?|flac|mp3|ogg)$/i.test(name)?'unsupported':'loaded';e.switches.unsupportedNext=false;
      asset.cached=asset.state==='loaded';e.emit('asset',clone(asset));assets.push(clone(asset));
    }
    return assets;
  }
  async relink(id,uri){
    const e=this.engine;e.call('relink',[id,uri]);const epoch=e.epoch,asset=entity(e.patch.assets,id);
    asset.state='loading';asset.progress=0;e.emit('asset',clone(asset));await e.wait(this.loadMs);
    if(e.epoch!==epoch)throw new Error('Patch replaced during relink');
    asset.uri=uri;asset.state=/\.(wav|aiff?|flac|mp3|ogg)$/i.test(uri)?'loaded':'unsupported';asset.progress=1;asset.cached=asset.state==='loaded';e.emit('asset',clone(asset));
  }
  getPeaks(id,bins){
    const e=this.engine;entity(e.patch.assets,id);e.call('getPeaks',[id,bins]);
    if(!Number.isInteger(bins)||bins<1||bins>16384)throw new Error('Invalid peak count');
    const asset=entity(e.patch.assets,id);
    const source=asset.render?this.getPeaks(asset.render.source,Math.max(bins,600)):null;
    let seed=id==='k60f'?11:Array.from(id).reduce((n,c)=>n+c.charCodeAt(0),1);
    return Float32Array.from({length:bins},(_,i)=>{
      const t=i/bins;seed=(seed*1664525+1013904223)>>>0;
      const position=asset.render?.reverse?1-t:t;
      const random=()=>{seed=(seed*1664525+1013904223)>>>0;return seed/2**32;};
      let breakShape;
      if(id==='amen'){breakShape=.06*random();for(const h of [0,.125,.25,.3125,.5,.625,.75,.875])if(t>=h)breakShape=Math.max(breakShape,Math.exp(-(t-h)*(h%.25===0?22:40))*(h%.5===0?.95:.6)*(.5+.5*random()));}
      let value=source?source[Math.min(source.length-1,Math.round((asset.render.start+position*(asset.render.end-asset.render.start))*source.length))]:
        breakShape!=null?Math.min(1,breakShape):(t<.004?t/.004:1)*(Math.exp(-t*2.4)*.85+.06*Math.exp(-t*.6))*(.78+.22*seed/2**32);
      if(asset.render){const f=asset.render.fades,len=asset.render.end-asset.render.start;value*=Math.min(1,f.in?t*len/f.in:1,f.out?(1-t)*len/f.out:1);}
      return value;
    });
  }
  analyse(assetId,kind){
    const e=this.engine,asset=entity(e.patch.assets,assetId);
    if(!['transients','pitch','loops','loudness'].includes(kind)||asset.state!=='loaded')throw new Error('Analysis requires a loaded asset and supported kind');
    e.call('analyse',[assetId,kind]);const job={id:e.id('job'),assetId,kind,state:'running',progress:0,result:null};
    this.jobs.set(job.id,job);const fail=e.switches.failNextAnalysis;e.switches.failNextAnalysis=false;e.emit('job',clone(job));
    this.run(job,fail).catch(error=>{if(!e.closed&&job.state==='running'){job.state='failed';job.message=error.message;e.emit('job',clone(job));}});
    return clone(job);
  }
  async run(job,fail){
    const e=this.engine;
    for(const progress of [.24,.48,.76,1]){await e.wait(this.analysisMs/4);if(job.state!=='running')return;job.progress=progress;e.emit('job',clone(job));}
    if(fail){job.state='failed';job.message='Mock decoder stopped at 0.82 s';}
    else {job.state='done';job.result=job.kind==='transients'?[0,.0625,.125,.1875,.25,.3125,.375,.4375,.5,.5625,.625,.6875,.75,.8125,.875]:job.kind==='pitch'?{note:60,cents:3}:job.kind==='loops'?[{start:.55,end:.82,crossfade:.025},{start:.44,end:.71,crossfade:.02},{start:.62,end:.89,crossfade:.03}]:{lufs:-18.4,peak:-1.2};}
    e.emit('job',clone(job));
  }
  cancel(id){const job=this.jobs.get(id);if(!job)throw new Error('Unknown job');this.engine.call('cancel',[id]);if(job.state==='running'){job.state='cancelled';this.engine.emit('job',clone(job));}}
  cancelAll(){for(const job of this.jobs.values())if(job.state==='running')job.state='cancelled';}
}
