import React,{useEffect,useState} from 'react';
import * as DD from './design-system/index.jsx';
export function LogDrawer({getEvents,onClear,switches,onSwitch}){
  const [filter,setFilter]=useState(''),[events,setEvents]=useState(()=>getEvents().slice(-1000));
  useEffect(()=>{const timer=setInterval(()=>setEvents(getEvents().slice(-1000)),100);return()=>clearInterval(timer);},[getEvents]);
  const controls=[['failNextAnalysis','Fail next analysis'],['missing','Mark file missing'],['host','Host automation'],['unsupportedNext','Next file unsupported']];
  return <section className="log-drawer" aria-label="Engine event log">
    <div className="log-toolbar">
      <strong>Event log</strong><input aria-label="Filter event log" value={filter} onChange={e=>setFilter(e.target.value)} placeholder="Filter calls and events"/>
      <DD.Button size="sm" onClick={()=>{onClear();setEvents([]);}}>Clear</DD.Button>
      {controls.map(([key,label])=><DD.Toggle key={key} compact label={label} checked={!!switches?.[key]} onChange={value=>onSwitch(key,value)}/>)}
    </div>
    <div className="log-rows" role="log">
      {events.filter(e=>JSON.stringify(e).toLowerCase().includes(filter.toLowerCase())).map((event,index)=><div key={index} className="log-row" title={JSON.stringify(event.result??event.args)}>
        <time>{new Date(event.time).toLocaleTimeString()}</time><span>{event.kind}</span><b>{event.name}</b><code>{JSON.stringify(event.result??event.args)}</code>
      </div>)}
    </div>
  </section>;
}
