import {clone} from './validation.mjs';
const same=(a,b)=>JSON.stringify(a)===JSON.stringify(b);
const lists=['nodes','assets','regions','rules','modules','modulators','routes','macros','buses','chains','history'];
const dictionaries=['params','selectors'];
const object=v=>v&&typeof v==='object'&&!Array.isArray(v);
function changedProperties(before,after,path=[],out=[]){
  if(same(before,after))return out;
  if(object(before)&&object(after)){
    for(const key of new Set([...Object.keys(before),...Object.keys(after)]))changedProperties(before[key],after[key],[...path,key],out);
  }else if(Array.isArray(before)&&Array.isArray(after)&&before.every(v=>v?.id)&&after.every(v=>v?.id)&&before.length===after.length&&before.every(v=>after.some(n=>n.id===v.id))){
    for(const value of after)changedProperties(before.find(v=>v.id===value.id),value,[...path,{id:value.id}],out);
    if(!same(before.map(v=>v.id),after.map(v=>v.id)))out.push({path,value:after.map(v=>v.id),present:true,order:true});
  }else out.push({path,value:after===undefined?null:clone(after),present:after!==undefined});
  return out;
}
/** Retain only changed entities and order, never a patch snapshot in undo history. */
export function entityDelta(before,after){
  const changes=[];
  for(const collection of lists){
    const a=before[collection]??[],b=after[collection]??[];
    for(const id of new Set([...a,...b].map(x=>x.id))){
      const old=a.find(x=>x.id===id),value=b.find(x=>x.id===id);
      if(old&&value)changes.push(...changedProperties(old,value).map(change=>({kind:'property',collection,id,...change})));
      else if(!same(old,value))changes.push({kind:'entity',collection,id,value:clone(value??null),index:b.findIndex(x=>x.id===id)});
    }
    if(!same(a.map(x=>x.id),b.map(x=>x.id)))changes.push({kind:'order',collection,ids:b.map(x=>x.id)});
  }
  for(const collection of dictionaries){
    for(const id of new Set([...Object.keys(before[collection]),...Object.keys(after[collection])])){
      const old=before[collection][id],value=after[collection][id];
      if(old&&value)changes.push(...changedProperties(old,value).map(change=>({kind:'property',collection,id,...change})));
      else if(!same(old,value))changes.push({kind:'dictionary',collection,id,value:clone(value??null)});
    }
  }
  for(const key of Object.keys(after).filter(key=>!lists.includes(key)&&!dictionaries.includes(key))){
    if(!same(before[key],after[key]))changes.push({kind:'field',key,value:clone(after[key])});
  }
  return changes;
}
export function applyEntityDelta(patch,changes){
  for(const change of changes){
    const {kind,collection,id,value}=change;
    if(kind==='property'){
      if(!lists.includes(collection)&&!dictionaries.includes(collection))throw new Error('Unknown property collection');
      let target=Array.isArray(patch[collection])?patch[collection].find(v=>v.id===id):patch[collection][id];
      if(!target||!change.path.length)throw new Error('Unknown property owner');
      for(const part of change.path.slice(0,-1)){
        if(typeof part==='string'&&['__proto__','constructor','prototype'].includes(part))throw new Error('Invalid property path');
        target=typeof part==='object'?target.find(v=>v.id===part.id):target[part];if(!target)throw new Error('Unknown nested property owner');
      }
      const key=change.path.at(-1);if(typeof key!=='string'||['__proto__','constructor','prototype'].includes(key))throw new Error('Invalid property key');
      if(change.order){const items=target[key];if(!Array.isArray(items)||items.length!==value.length||new Set(value).size!==value.length||value.some(id=>!items.some(v=>v.id===id)))throw new Error('Invalid nested entity order');target[key]=value.map(id=>items.find(v=>v.id===id));}
      else if(change.present)target[key]=clone(value);else delete target[key];
    }else if(kind==='entity'){
      if(!lists.includes(collection))throw new Error('Unknown entity collection');
      const list=patch[collection];const index=list.findIndex(x=>x.id===id);
      if(index>=0)list.splice(index,1);
      if(value!==null)list.splice(Math.max(0,Math.min(list.length,change.index)),0,clone(value));
    }else if(kind==='order'){
      if(!lists.includes(collection)||new Set(change.ids).size!==change.ids.length)throw new Error('Invalid entity order');
      const items=patch[collection];if(change.ids.length!==items.length||change.ids.some(id=>!items.some(x=>x.id===id)))throw new Error('Invalid entity order');
      patch[collection]=change.ids.map(id=>items.find(x=>x.id===id));
    }else if(kind==='dictionary'){
      if(!dictionaries.includes(collection)||['__proto__','constructor','prototype'].includes(id))throw new Error('Invalid dictionary entry');
      if(value===null)delete patch[collection][id];else patch[collection][id]=clone(value);
    }else if(kind==='field'){
      if(['__proto__','constructor','prototype'].includes(change.key)||!Object.hasOwn(patch,change.key))throw new Error('Unknown patch field');
      patch[change.key]=clone(value);
    }else throw new Error('Unknown undo operand');
  }
}
