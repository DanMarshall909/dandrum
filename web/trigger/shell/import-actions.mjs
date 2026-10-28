import {interpretFilename,mappingAssignments} from '../engine/filename-mapping.mjs';
export async function selectNativeSamples(presenter,options={}){
  try{
    const files=await presenter.props.store.engine.chooseSamples();
    if(presenter.props.store.closed)return;
    if(files.length)return presenter.queueImport(files,options);
  }catch(error){if(!presenter.props.store.closed)presenter.setState({lastImportError:error.message??error.code??String(error)});}
}
export const importActions={
  async cancelImport(dialog=this.state.importDialog){
    if(this.props.store.closed||!dialog||this.state.importDialog!==dialog)return;
    // Clear synchronously so repeated or obsolete callbacks cannot release twice.
    this.setState({importDialog:null});
    if(this.props.store.engine.releaseSamples)try{await this.props.store.engine.releaseSamples(dialog.files);}
    catch(error){if(!this.props.store.closed){const message=error.message??error.code??String(error);this.setState({lastImportError:message,modNote:message});}}
  },
  queueImport(files,options={}){
    if(this.props.store.closed)return;
    const refs=Array.from(files,file=>({...file,name:file.name,uri:file.uri??'mock://'+file.name}));
    if(!refs.length)return;
    if(this.state.importDialog)return this.cancelImport().then(()=>this.queueImport(refs,options));
    this.setState({lastImportError:null});
    const engine=this.props.store.engine,native=engine.backendId==='juce-free',patch=this.props.store.getSnapshot().patch;
    const group=native?patch.nodes.find(node=>node.kind==='group'):null;
    const defaults=native?{group:group?.id,groupName:group?.name??patch.name}:{};
    if(refs.length===1){
      let startNote=options.startNote;
      if(native&&startNote==null&&!options.replace&&!options.target){
        startNote=Array.from({length:128},(_,i)=>(48+i)%128).find(note=>!patch.rules.some(rule=>rule.source&&note>=rule.noteLo&&note<=rule.noteHi));
        if(startNote==null){
          this.setState({lastImportError:'No free MIDI note; replace an existing sample or change a mapping.'});
          return engine.releaseSamples(refs).catch(error=>{if(!this.props.store.closed)this.setState({lastImportError:error.message??error.code??String(error)});});
        }
      }
      return this.importFiles(refs,{policy:native?'sequential':'stack',...defaults,...options,...(startNote==null?{}:{startNote})});
    }
    const recognized=refs.every(file=>interpretFilename(file.name).root!=null);
    this.setState({importDialog:{files:refs,policy:recognized?'root-velocity':'sequential',startNote:options.startNote??48,groupName:'Imported',...defaults,...options},cmenu:null});
  },
  async importFiles(files,options){
    if(this.props.store.closed)return;
    this.setState({importDialog:null,importing:true,lastImportError:null});
    try{
      const result=await this.props.store.operation(options.replace?'Replace sample':'Import samples','importSamples',files,options);
      if(this.props.store.closed)return;
      if(result.warning)this.setState({modNote:result.warning});
      const unsupported=result.assets.filter(a=>a.state==='unsupported');
      if(result.rule&&!options.target)this.setState({zoneSel:result.rule,st:'sample',browser:false});
      if(unsupported.length)this.setState({lastImportError:unsupported.map(a=>a.name).join(', ')+' could not be loaded. Use WAV, AIFF, FLAC, MP3 or OGG.'});
    }catch(error){if(!this.props.store.closed){const message=error.message??error.code??String(error);this.setState({modNote:message,lastImportError:message});}}
    finally{if(!this.props.store.closed)this.setState({importing:false});}
  },
  importModel(model){
    const dialog=this.state.importDialog;
    model.onFiles=files=>{const relink=this.state.relinkTarget;if(relink&&files[0]){this.setState({relinkTarget:null});return this.command('Relink source','relink',relink,files[0].uri??'mock://'+files[0].name);}
      const replace=this.state.replaceTarget;this.setState({replaceTarget:null});return this.queueImport(files,replace?{replace}:{});};
    model.onSampleDrop=e=>{if(e.dataTransfer.files.length){e.preventDefault();e.stopPropagation();this.queueImport(e.dataTransfer.files,{replace:model.regionId});return;}
      const source=this.state.dragSrc,patch=this.props.store.getSnapshot().patch,region=patch.regions.find(r=>r.id===source?.id),asset=patch.assets.find(a=>a.id===(region?.assetId??source?.id));if(!asset)return;e.preventDefault();e.stopPropagation();this.command('Replace sample','replaceRegionAsset',model.regionId,asset.id);this.setState({dragSrc:null});};
    model.onFileDrag=e=>{if(Array.from(e.dataTransfer?.types??[]).includes('Files')){e.preventDefault();e.dataTransfer.dropEffect='copy';}};
    model.onFileDrop=e=>{if(!e.dataTransfer?.files.length)return;e.preventDefault();e.stopPropagation();this.queueImport(e.dataTransfer.files);};
    model.importGroupNameEditable=this.props.store.engine.backendId!=='juce-free';
    model.cancelImport=()=>this.cancelImport(dialog);
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
