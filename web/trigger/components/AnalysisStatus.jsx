import React from 'react';
import * as DD from './design-system/index.jsx';
export function AnalysisStatus({job}){
  if(!job)return null;
  return <div style={{display:'flex',gap:8,alignItems:'center',flexWrap:'wrap',padding:'6px 8px',background:'var(--dd-ink-3)',borderRadius:4}}>
    <DD.StatusMessage compact kind={job.state==='failed'?'error':job.state==='running'?'busy':'info'} title={job.kind+' · '+(job.state==='running'?Math.round(job.progress*100)+'%':job.state)}>
      {job.state==='failed'?job.message+' · Existing edits are unchanged.':job.state==='running'?'You can keep editing and playing.':job.state==='cancelled'?'Existing edits are unchanged.':'Analysis is ready.'}
    </DD.StatusMessage>
    {job.state==='running'?<DD.Button size="sm" onClick={job.cancel}>Cancel analysis</DD.Button>:<>
      {job.apply&&<DD.Button size="sm" variant="primary" onClick={job.apply}>Apply result</DD.Button>}
      <DD.Button size="sm" variant="ghost" onClick={job.discard}>Dismiss</DD.Button>
    </>}
  </div>;
}
