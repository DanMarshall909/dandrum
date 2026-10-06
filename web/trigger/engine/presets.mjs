/** The supplied Felt Kit data. These are mock resources, never decoded audio. */
import {defaultParameters,initialiseModule} from './voice-defaults.mjs';
export const noteName = note => ['C','C♯','D','D♯','E','F','F♯','G','G♯','A','A♯','B'][note % 12] + (Math.floor(note / 12) - 1);
/** @returns {import('./contract').Patch} */
export const emptyPatch = () => ({
  id: 'untitled', name: 'Untitled', nodes: [{id:'root',name:'Untitled',kind:'instrument',parent:null}],
  assets: [], regions: [], rules: [], selectors: {}, modules: [], params: {},
  modulators: [], routes: [], macros: Array.from({length:8},(_,i)=>({id:`Macro ${i+1}`,name:`Macro ${i+1}`,value:0,default:0,bindings:[],host:false,midi:null})), buses: [{id:'main',name:'Main',channels:[1,2]}],
  chains: [], history: [], midiLearn:null, voicePolicy: {mode:'poly',limit:32,steal:'oldest',sameNote:'retrigger',glide:0},
});

export function feltKit() {
  const patch = emptyPatch();
  patch.id = 'felt-kit'; patch.name = 'Felt Kit'; patch.nodes[0].name = patch.name;
  const addNode = (id,name,kind,parent,extra={}) => patch.nodes.push({id,name,kind,parent,...extra});
  addNode('keys','Keys','group','root'); addNode('drums','Drums','group','root'); addNode('break','Break','group','root');
  const sliceNames = ['Kick 1','Hat 1','Snare 1','Ghost','Kick 2','Hat 2','Snare 2','Hat 3'];
  const starts = [0,.125,.25,.3125,.5,.625,.75,.875];
  const asset = (id,name,duration,group) => {
    patch.assets.push({id,name,uri:`mock://${name}`,state:'loaded',cached:true,duration,sampleRate:48000,bits:24,channels:2,group,analysis:group==='keys'?{note:Number(id.match(/[0-9]+/)?.[0]??60),cents:3,lufs:-18.4,peak:-1.2}:undefined});
    if(group==='drums')patch.regions.push({id,assetId:id,name,kind:'sample',start:0,end:1,fades:{in:0,out:0},loop:null,playback:'once',root:id==='kick'?36:id==='chat'?42:id==='ohat'?46:38});
  };
  asset('amen','amen_172.wav',1.395,'break');
  patch.regions.push({id:'amen',assetId:'amen',name:'amen_172.wav',kind:'sample',start:0,end:1,root:24,playback:'once',fades:{in:0,out:0},loop:null});
  sliceNames.forEach((name,i) => {
    const id=`slice-${i}`;
    patch.regions.push({id,assetId:'amen',name,kind:'slice',start:starts[i],end:starts[i+1]??1,note:24+i,playback:name.startsWith('Hat')?'gated':'once',tune:i===3?2:0,gain:i===3?-6:0,pan:i===1||i===5?-.18:0,choke:name.startsWith('Hat')?1:0});
    patch.rules.push({id:`sl${i}`,group:'break',name,source:id,noteLo:24+i,noteHi:24+i,velLo:1,velHi:127,root:24+i});
  });
  for(const [id,name,note] of [['kick','Kick',36],['snare','Snare',38],['chat','Closed Hat',42],['ohat','Open Hat',46]]) {
    addNode(id,name,'sound','drums',{note,choke:id==='chat'||id==='ohat'?1:0});
    if(id==='snare') continue;
    asset(id,`${id}.wav`,.6,'drums');
    patch.rules.push({id,group:'drums',target:id,name,source:id,noteLo:note,noteHi:note,velLo:1,velHi:127,root:note});
  }
  patch.rules.push({id:'snS',group:'drums',target:'snare',name:'Snare soft',source:'snare-soft',noteLo:38,noteHi:38,velLo:1,velHi:95,root:38},
    {id:'snH',group:'drums',target:'snare',name:'Snare hard ×3',source:'snare-hard',noteLo:38,noteHi:38,velLo:96,velHi:127,root:38});
  for(const [id,name] of [['snare-soft','snare_soft.wav'],['hard-a','snare_hard_a.wav'],['hard-b','snare_hard_b.wav'],['hard-c','snare_hard_c.wav']]) asset(id,name,.5,'drums');
  patch.selectors['snare-hard']={policy:'rr',reset:'per-note',candidates:['hard-a','hard-b','hard-c'].map((id,i)=>({id,name:`Hard ${'ABC'[i]}`,weight:i===0?2:1,muted:false,solo:false,plays:0}))};
  patch.selectors.snare={policy:'velocity',softHi:101,hardLo:90,curve:'equal-power',candidates:[{id:'snare-soft',name:'Soft',weight:1,muted:false,solo:false},{id:'snare-hard',name:'Hard',weight:1,muted:false,solo:false}]};
  for(const [root,lo,hi] of [[48,48,53],[55,54,59],[60,60,65],[66,66,71],[72,72,96]]) {
    addNode(`k${root}`,`Felt ${noteName(root)}`,'sound','keys',{root});
    for(const [dynamic,velLo,velHi] of [['p',1,90],['f',81,127]]) {
      const id=`k${root}${dynamic}`,name=`Felt ${noteName(root)} ${dynamic}`;
      asset(id,`felt_${noteName(root).replace('♯','#')}_${dynamic}.wav`,4.82,'keys');
      patch.regions.push({id,assetId:id,name,kind:'sample',start:.012,end:.94,fades:{in:.004,out:.08},loop:null,playback:'once',root});
      patch.rules.push({id,group:'keys',target:`k${root}`,name,source:id,noteLo:lo,noteHi:hi,velLo,velHi,root});
    }
  }
  patch.params=defaultParameters();
  patch.macros=[['Tone',.56],['Body',.4],['Space',.32],['Drive',.18],['Snap',.64],['Width',.5],['Macro 7',0],['Macro 8',0]].map(([name,value],i)=>({id:name,name,value,default:value,bindings:i<6?[{destination:i===0?'Cutoff':i===1?'Level':name,range:[0,1]}]:[],host:false,midi:null}));
  patch.modulators=[['Amp envelope','envelope',null],['Filter envelope','envelope','A'],['LFO 1','lfo','B'],['LFO 2','lfo',null],['Velocity','performance','C'],['Key track','performance',null],['Aftertouch','performance',null],['Mod wheel · CC 1','performance',null],['Pitch bend','performance',null],['Random per note','random',null],['Macro · Tone','macro','D'],['Macro · Space','macro',null]].map(([id,kind,slot])=>({id,name:id,kind,slot,shape:kind==='lfo'?'sine':kind==='envelope'?'adsr':'linear',config:{attack:.012,decay:.38,sustain:.5,release:1.2,rate:2.4,phase:0,smooth:0},locked:id==='Amp envelope'}));
  Object.assign(patch.modulators.find(m=>m.id==='Filter envelope').config,{attack:.001,decay:.24,sustain:.3,release:.42});
  const routeDefs=[['Amp envelope','Amp gain',1,'Uni','Exp'],['Filter envelope','Filter cutoff',.42,'Uni','Linear'],['LFO 1','Pitch',.05,'Bi','Linear'],['LFO 2','Filter drive',.18,'Bi','Linear',false],['Velocity','Amp gain',.3,'Uni','Exp'],['Velocity','Filter cutoff',.15,'Uni','Linear'],['Key track','Filter cutoff',.5,'Bi','Linear'],['Aftertouch','Sample start',.15,'Uni','Linear'],['Mod wheel · CC 1','LFO 1 depth',1,'Uni','Linear'],['Pitch bend','Pitch',.02,'Bi','Linear'],['Random per note','Pan',.12,'Bi','Linear'],['Macro · Tone','Filter cutoff',.2,'Uni','Linear'],['Macro · Tone','Filter resonance',-.1,'Uni','Linear'],['Macro · Space','Send · Reverb',.6,'Uni','S-curve']];
  patch.routes=routeDefs.map(([source,destination,amount,polarity,curve,on=true],i)=>({id:`route-${i}`,source,destination,amount,polarity,curve,on,locked:i===0}));
  patch.modules=[{id:'sample',type:'Sample player',params:{mode:'once'}},{id:'lush',type:'Lush',params:{mode:'note',level:.55,width:.7}},{id:'filter',type:'Filter',params:{type:'lp',slope:24,cutoff:.42,reso:.22}},{id:'amp',type:'Amplifier',params:{level:.62,pan:.5}}];
  patch.modules.forEach(module=>initialiseModule(patch,module));
  patch.nodes.forEach(n=>{n.output=n.id==='drums'?'drums':n.id==='snare'?'snare':n.id==='root'?'main':'inherit';n.sends={rev:n.id==='snare'?.3:n.id==='keys'?.18:0,dly:n.id==='snare'?.18:0};});
  patch.buses=[{id:'main',name:'Main',channels:[1,2],level:0},{id:'drums',name:'Drums',channels:[3,4],level:-1.5},{id:'snare',name:'Snare',channels:[5,6],level:0}];
  patch.history=[{id:'fade',name:'Fade',on:true,config:{in:.004,out:.08}},{id:'norm',name:'Normalize',on:false,config:{target:-1}},{id:'trim',name:'Trim',on:true,config:{start:.012,end:.94}},{id:'src',name:'Source',on:true,locked:true,config:{asset:'k60f'}}];
  patch.history.forEach(op=>op.regionId='k60f');
  patch.history.push({id:'amen-slices',regionId:'amen',name:'Slice',on:true,config:{count:8,method:'even'}},{id:'amen-trim',regionId:'amen',name:'Trim',on:true,config:{start:0,end:1}},{id:'amen-source',regionId:'amen',name:'Source',on:true,locked:true,config:{asset:'amen'}});
  patch.chains=[{id:'drums',name:'Drums',output:'drums',modules:[{id:'eq',type:'EQ',params:{Low:2,High:1.5}},{id:'comp',type:'Compressor',params:{Threshold:-18,Ratio:4,Attack:10,Release:120,Makeup:3,Mix:100}},{id:'sat',type:'Saturate',params:{Drive:30,Mix:60}}]}, {id:'keys',name:'Keys',output:'main',modules:[{id:'eq2',type:'EQ',params:{Low:-1.5}}]}, {id:'brk',name:'Break',output:'main',modules:[{id:'crush',type:'Bitcrush',params:{Bits:12},bypassed:true}]}, {id:'rev',name:'Reverb',output:'main',modules:[{id:'conv',type:'Convolution',params:{Predelay:12,Mix:100}},{id:'eqr',type:'EQ',params:{'Low cut':180}}]}, {id:'dly',name:'Delay',output:'main',modules:[{id:'delay',type:'Delay',params:{Time:261,Feedback:35}},{id:'flt',type:'Filter',params:{Cutoff:3200}}]}];
  const chainDetails={drums:['Kick · Snare · Hats',-1.5],keys:['Felt C3–C5 · 10 zones',0],brk:['amen_172.wav · 8 slices',-3],rev:['Send A · plate_small.wav',-4],dly:['Send B · 1/8 dotted',-8]};
  for(const chain of patch.chains){[chain.detail,chain.level]=chainDetails[chain.id];}
  return patch;
}
