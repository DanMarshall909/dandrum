import {interpretFilename,mappingAssignments} from '../engine/filename-mapping.mjs';
export const importActions={
  queueImport(files,options={}){
    const refs=Array.from(files,file=>({name:file.name,uri:file.uri??'mock://'+file.name}));
    if(!refs.length)return;
    this.setState({lastImportError:null});
    if(refs.length===1)return this.importFiles(refs,{policy:'stack',...options});
    const recognized=refs.every(file=>interpretFilename(file.name).root!=null);
    this.setState({importDialog:{files:refs,policy:recognized?'root-velocity':'sequential',startNote:options.startNote??48,groupName:'Imported',...options},cmenu:null});
  },
  async importFiles(files,options){
    this.setState({importDialog:null,importing:true,lastImportError:null});
    try{
      const result=await this.props.store.operation(options.replace?'Replace sample':'Import samples','importSamples',files,options);
      const unsupported=result.assets.filter(a=>a.state==='unsupported');
      if(result.rule&&!options.target)this.setState({zoneSel:result.rule,st:'sample',browser:false});
      if(unsupported.length)this.setState({lastImportError:unsupported.map(a=>a.name).join(', ')+' could not be loaded. Use WAV, AIFF, FLAC, MP3 or OGG.'});
    }catch(error){this.setState({modNote:error.message,lastImportError:error.message});}
    finally{this.setState({importing:false});}
  },
  importModel(model){
    const dialog=this.state.importDialog;
    model.onFiles=files=>{const relink=this.state.relinkTarget;if(relink&&files[0]){this.setState({relinkTarget:null});return this.command('Relink source','relink',relink,'mock://'+files[0].name);}
      const replace=this.state.replaceTarget;this.setState({replaceTarget:null});return this.queueImport(files,replace?{replace}:{});};
    model.onSampleDrop=e=>{if(e.dataTransfer.files.length){e.preventDefault();e.stopPropagation();this.queueImport(e.dataTransfer.files,{replace:model.regionId});return;}
      const source=this.state.dragSrc,patch=this.props.store.getSnapshot().patch,region=patch.regions.find(r=>r.id===source?.id),asset=patch.assets.find(a=>a.id===(region?.assetId??source?.id));if(!asset)return;e.preventDefault();e.stopPropagation();this.command('Replace sample','replaceRegionAsset',model.regionId,asset.id);this.setState({dragSrc:null});};
    model.onFileDrag=e=>{if(Array.from(e.dataTransfer?.types??[]).includes('Files')){e.preventDefault();e.dataTransfer.dropEffect='copy';}};
    model.onFileDrop=e=>{if(!e.dataTransfer?.files.length)return;e.preventDefault();e.stopPropagation();this.queueImport(e.dataTransfer.files);};
    model.cancelImport=()=>this.setState({importDialog:null});
    model.setImportOption=(key,value)=>this.setState({importDialog:{...this.state.importDialog,[key]:value}});
    model.applyImport=()=>this.importFiles(dialog.files,dialog);
    model.importOptions=dialog;
    if(dialog){model.isEmpty=false;model.status={kind:'info',title:'Choose mapping',text:dialog.files.length+' files'};}
    model.importSummary=dialog?dialog.files.map(file=>interpretFilename(file.name)):[];
    model.mappingPreview=[];model.importError=null;
    if(dialog){try{model.mappingPreview=mappingAssignments(dialog.files,dialog);}catch(error){model.importError=error.message;}}
    const policies=['sequential','root-velocity','stack','rr'];
    model.dropOpts=model.dropOpts.map((option,index)=>({...option,id:policies[index],selected:dialog?.policy===policies[index],
      select:()=>model.setImportOption('policy',policies[index])}));
    if(this.state.importing)model.status={kind:'busy',title:'Loading',text:'Importing samples'};
    if(this.state.lastImportError){model.unsup=true;model.banner=true;model.unsupportedMessage=this.state.lastImportError;model.status={kind:'warn',title:'Import failed',text:this.state.lastImportError};}
  },
};
