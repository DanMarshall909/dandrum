const clamp=(v,lo,hi)=>Math.max(lo,Math.min(hi,v));
const attackX=value=>clamp(value<=.012?value/.012*6:6+(value-.012)*500,0,100);
const attackTime=x=>x<=6?x/6*.012:.012+(x-6)/500;
const decayWidth=value=>clamp(value<=.24?value/.24*38:38+(value-.24)/.14*16,1,150);
const decayTime=x=>x<=38?x/38*.24:.24+(x-38)/16*.14;
const releaseWidth=value=>clamp(value<=.42?value/.42*40:40+(value-.42)/.78*40,1,80);
const releaseTime=x=>x<=40?x/40*.42:.42+(x-40)/40*.78;

/** One schematic projection owns both the curve and its editable handles. */
export function envelopeGeometry(mod){
  const c=mod.config,amp=mod.id==='Amp envelope',shape=mod.shape;
  const level=amp?clamp(1+20*Math.log10(Math.max(.0001,c.sustain??.5))/18.8,0,1):c.sustain??.5;
  const a=attackX(c.attack??.01),hold=shape==='ahdsr'?clamp((c.hold??.2)*50,0,40):0,d=clamp(a+hold+decayWidth(c.decay??.3),a+hold+1,200),r=220+releaseWidth(c.release??1);
  let points=[[0,0],[a/300,1],...(hold?[[ (a+hold)/300,1]]:[]),[d/300,level],[220/300,level],[r/300,0]];
  let handles=[{key:'attack',x:a/300,y:0},...(hold?[{key:'hold',x:(a+hold)/300,y:0}]:[]),{key:'decay',x:d/300,y:1-level},{key:'sustain',x:220/300,y:1-level},{key:'release',x:r/300,y:1}];
  if(shape==='multi'){points=c.points??[[0,0],[.08,1],[.22,.3],[.38,.8],[.7,.4],[1,0]];handles=points.map(([x,v],i)=>({key:'point-'+i,x,y:1-v,value:v}));}
  const valueFor=(key,x,y)=>key==='sustain'?(amp?Math.pow(10,-(1-clamp(1-y,0,1))*18.8/20):clamp(1-y,0,1)):
    key==='attack'?attackTime(clamp(x*300,0,100)):key==='hold'?clamp((x*300-a)/50,0,10):key==='decay'?decayTime(clamp(x*300-a-hold,1,150)):releaseTime(clamp(x*300-220,1,80));
  return {points,handles,valueFor,sustainX:220,pointsText:points.map(([x,v])=>(x*300)+','+(100-v*100)).join(' ')};
}
