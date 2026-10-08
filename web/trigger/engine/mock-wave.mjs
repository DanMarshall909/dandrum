/** Deterministic signed test signal; no browser audio or external decoding. */
const cache=new Map(),frames=4096;
export function mockSampleAt(id,index){
  const seed=Array.from(id).reduce((n,c)=>n+c.charCodeAt(0),11);
  const t=index/frames;
  return Math.sin(t*(seed%73+37)*Math.PI*2)*Math.exp(-t*2.4);
}
export function zeroCrossings(id){
  if(!cache.has(id)){
    const positions=[0];let previous=mockSampleAt(id,0);
    for(let index=1;index<=frames;index++){
      const value=mockSampleAt(id,index);
      if(previous*value<0)positions.push((index-1+Math.abs(previous)/(Math.abs(previous)+Math.abs(value)))/frames);
      previous=value;
    }
    cache.set(id,positions);
  }
  return cache.get(id);
}
export function snapZero(id,position){
  return zeroCrossings(id).reduce((closest,value)=>Math.abs(value-position)<Math.abs(closest-position)?value:closest,0);
}
