const pitch={C:0,D:2,E:4,F:5,G:7,A:9,B:11};
const dynamics={ppp:16,pp:32,p:48,mp:64,mf:80,f:96,ff:112,fff:127};
export function interpretFilename(name){
  const stem=name.replace(/\.[^.]+$/,''),boundary='(?:^|[ _.-])';
  const note=stem.match(new RegExp(boundary+'([A-Ga-g])([#♯b♭]?)(-?\\d)(?=$|[ _.-])'));
  const midi=stem.match(new RegExp(boundary+'(?:midi|note|key)[ _-]?(\\d{1,3})(?=$|[ _.-])','i'));
  let root=midi?Number(midi[1]):note?(Number(note[3])+1)*12+pitch[note[1].toUpperCase()]+(['#','♯'].includes(note[2])?1:['b','♭'].includes(note[2])?-1:0):null;
  if(root!=null&&(root<0||root>127))root=null;
  const vel=stem.match(new RegExp(boundary+'(?:vel(?:ocity)?|v)[ _-]?(\\d{1,3})(?=$|[ _.-])','i'));
  const dynamic=stem.match(new RegExp(boundary+'(ppp|fff|pp|ff|mp|mf|p|f)(?=$|[ _.-])','i'));
  const velocity=vel?Number(vel[1]):dynamic?dynamics[dynamic[1].toLowerCase()]:null;
  const rr=stem.match(new RegExp(boundary+'(?:rr|roundrobin)[ _-]?(\\d+)(?=$|[ _.-])','i'));
  return {name,root,velocity:velocity!=null&&velocity>=1&&velocity<=127?velocity:null,rr:rr?Number(rr[1]):null,
    dynamic:dynamic?.[1].toLowerCase()??null};
}

export function mappingAssignments(files,{policy='sequential',startNote=48}={}){
  if(!['sequential','root-velocity','stack','rr'].includes(policy))throw new Error('Unknown import mapping');
  if(!Number.isInteger(startNote)||startNote<0||startNote>127)throw new Error('Invalid starting note');
  if(policy==='sequential'&&startNote+files.length>128)throw new Error('Files exceed the MIDI keyboard');
  const rows=files.map((file,index)=>({...interpretFilename(file.name),id:file.id,index}));
  if(policy!=='root-velocity')return rows.map((row,index)=>({...row,root:row.root??(policy==='sequential'?startNote+index:startNote),
    noteLo:policy==='stack'?24:policy==='sequential'?startNote+index:startNote,
    noteHi:policy==='stack'?96:policy==='sequential'?startNote+index:startNote,velLo:1,velHi:127}));
  const roots=[...new Set(rows.map(row=>row.root??startNote+row.index))].sort((a,b)=>a-b);
  if(roots.some(root=>root>127))throw new Error('Files exceed the MIDI keyboard');
  return rows.map(row=>{
    const root=row.root??startNote+row.index,index=roots.indexOf(root);
    const layerVelocities=[...new Set(rows.filter(r=>(r.root??startNote+r.index)===root).map(r=>r.velocity??96))].sort((a,b)=>a-b);
    const velocity=row.velocity??96,layer=layerVelocities.indexOf(velocity);
    return {...row,root,noteLo:index?Math.floor((roots[index-1]+root)/2)+1:root,
      noteHi:index<roots.length-1?Math.floor((root+roots[index+1])/2):Math.max(root,96),
      velLo:layer?Math.floor((layerVelocities[layer-1]+velocity)/2)+1:1,
      velHi:layer<layerVelocities.length-1?Math.floor((velocity+layerVelocities[layer+1])/2):127};
  });
}
