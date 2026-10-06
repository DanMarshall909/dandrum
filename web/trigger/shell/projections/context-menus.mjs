// Frame status seed; operation menus are built from current adapter entities.
export function contextMenusProjection(context){
  const {isMin,empty,drop,missing,loading,tr}=context;
  const status=empty||drop?{kind:'info',title:'Empty',text:'Drop or browse samples'}:loading?{kind:'busy',title:'Loading',text:'Preparing asset'}:missing?{kind:'error',title:'File missing',text:'Locate the source'}:tr==='run'?{kind:'busy',title:'Analysing',text:'Analysis in progress'}:tr==='fail'?{kind:'warn',title:'Analysis failed',text:'Regions preserved'}:{kind:'ok',title:'Ready',text:isMin?'':'Ready to play'};
  Object.assign(context,{KM:this.state.kmenu,CM:this.state.cmenu,DM:this.state.dmenu,kmenus:[],status});
}
