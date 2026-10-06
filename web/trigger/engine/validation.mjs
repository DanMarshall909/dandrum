export const clone=value=>structuredClone(value);
export function entity(list,id){const item=list.find(x=>x.id===id);if(!item)throw new Error(`Unknown entity: ${id}`);return item;}
export function named(value){if(typeof value!=='string'||!value.trim())throw new Error('A name is required');return value.trim();}
export function finite(value){if(!Number.isFinite(value))throw new Error('Value must be finite');return value;}
export const bounded=(value,min,max)=>Math.max(min,Math.min(max,finite(value)));
export function reorder(list,order){
  if(new Set(order).size!==list.length||order.length!==list.length||order.some(id=>!list.some(x=>x.id===id)))throw new Error('Order must contain every ID once');
  return order.map(id=>entity(list,id));
}
export function noteRange(rule){
  for(const [lo,hi,min,max] of [['noteLo','noteHi',0,127],['velLo','velHi',1,127]]){
    if(!Number.isInteger(rule[lo])||!Number.isInteger(rule[hi])||rule[lo]<min||rule[hi]>max||rule[lo]>rule[hi])throw new Error(`Invalid ${lo}/${hi} range`);
  }
  if(!Number.isInteger(rule.root)||rule.root<0||rule.root>127)throw new Error('Invalid root');
  return rule;
}
export function regionRange(region){
  if(finite(region.start)<0||finite(region.end)>1||region.start>=region.end)throw new Error('Invalid region range');
  if(region.fades){for(const field of ['in','out']){if(finite(region.fades[field])<0)throw new Error('Invalid fade range');}
    region.fades={in:Math.min(region.fades.in,region.end-region.start),out:Math.min(region.fades.out,region.end-region.start)};}
  if(region.loop){
    if(finite(region.loop.start)<region.start||finite(region.loop.end)>region.end||region.loop.start>=region.loop.end||finite(region.loop.crossfade)<0)throw new Error('Invalid loop range');
    region.loop={...region.loop,crossfade:Math.min(region.loop.crossfade,region.loop.end-region.loop.start)};
  }
  if(region.root!=null&&(!Number.isInteger(region.root)||region.root<0||region.root>127))throw new Error('Invalid root');
  return region;
}
