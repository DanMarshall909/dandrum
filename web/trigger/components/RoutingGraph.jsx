import React from 'react';
export function RoutingGraph({rows,busses}){
  return <div role="img" aria-label="Routing graph" style={{display:'grid',gridTemplateColumns:'minmax(0,1fr) 30px minmax(0,1fr)',gap:6,padding:12,background:'var(--dd-ink-0)',borderRadius:4}}>
    {rows.map(row=><React.Fragment key={row.id}><span style={{color:'var(--dd-paper-2)',font:'500 12px var(--font-ui)',overflow:'hidden',textOverflow:'ellipsis'}}>{row.name}</span><span style={{color:'var(--dd-vermilion)'}}>→</span><span style={{font:'500 12px var(--font-value)',color:'var(--dd-paper-1)'}}>{busses.find(b=>b.id===row.effective)?.name??'Main'}</span></React.Fragment>)}
  </div>;
}
