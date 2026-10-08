// @ts-check
/** @type {import('./contract').Telemetry} */
export const emptyTelemetry={voices:{active:0,max:32},notes:[],playheads:[],selectorPos:{},mod:{},meters:{},host:{},hostValues:{}};
/** @param {import('./contract').EngineAdapter} engine */
export function telemetryReader(engine){
  let snapshot=emptyTelemetry;const listeners=/** @type {Set<()=>void>} */(new Set());
  const unsubscribe=engine.on('telemetry',value=>{snapshot=value;for(const listener of listeners)listener();});
  return {subscribe:/** @param {()=>void} listener */listener=>{listeners.add(listener);return()=>listeners.delete(listener);},getSnapshot:()=>snapshot,close:()=>{unsubscribe();listeners.clear();}};
}
