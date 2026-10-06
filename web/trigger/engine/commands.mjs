export function parameterCommand(_engine,id,value) {
  let before,captured=false;
  const command={id:`param:${id}`,label:`Change ${id}`,mergeKey:`param:${id}`,before,value,
    do(adapter){
      const previous=captured?before:adapter.getPatch().params[id]?.value;
      const result=adapter.setParam(id,value);
      if(!captured){before=previous;command.before=before;captured=true;}
      return result;
    },
    undo: adapter=>adapter.setParam(id,before),
    merge(next) {return {...next,before,undo:this.undo};}};
  return command;
}
