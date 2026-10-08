import React from 'react';
import * as DD from '../design-system/index.jsx';
export function DropPage({model}){
  if(!model.isDrop)return null;
  const options=model.importOptions;
  return <section className="import-choices" aria-label="Import mapping">
    <div style={{display:'flex',flexDirection:'column',alignItems:'center',gap:4}}>
      <strong style={{font:'700 13px var(--font-ui)',letterSpacing:'.1em',textTransform:'uppercase'}}>{options.files.length} files · choose mapping</strong>
      <span style={{font:'500 13px var(--font-ui)',color:'var(--dd-paper-2)'}}>Review the interpreted notes and velocities, then import.</span>
    </div>
    <div className="import-policy-grid">
      {model.dropOpts.map(option=><button key={option.id} onClick={option.select} aria-pressed={option.selected} className="import-policy"
        style={{borderColor:option.selected?'var(--dd-vermilion)':'var(--dd-line-2)'}}>
        <div style={{height:44,display:'grid',gridTemplateColumns:'repeat(6,1fr)',gridTemplateRows:'repeat(2,1fr)',gap:2,padding:4,background:'var(--dd-ink-0)',borderRadius:2}}>
          {option.cells.map((cell,index)=><div key={index} style={{gridColumn:cell.col,gridRow:cell.row,background:cell.bg,borderRadius:1}}/>)}
        </div>
        <span style={{font:'600 13px var(--font-ui)',color:'var(--dd-paper-1)'}}>{option.name}</span>
        <span style={{font:'500 12px/1.35 var(--font-ui)',color:'var(--dd-paper-3)'}}>{option.desc}</span>
      </button>)}
    </div>
    <div style={{display:'flex',gap:12,alignItems:'center'}}>
      <label style={{fontSize:12}}>Group <input aria-label="Import group name" value={options.groupName} onChange={e=>model.setImportOption('groupName',e.target.value)}/></label>
      <DD.NumericField label="Starting note" value={options.startNote} min={0} max={127} width={84} compact onChange={value=>model.setImportOption('startNote',value)}/>
    </div>
    <div className="import-preview" role="region" aria-label="Interpreted mapping">
      {model.mappingPreview.map((row,index)=><div key={index}>{row.name} → root {row.root}, keys {row.noteLo}–{row.noteHi}, velocity {row.velLo}–{row.velHi}{row.rr!=null?' · RR '+row.rr:''}</div>)}
      {model.importError&&<div role="alert">{model.importError}</div>}
    </div>
    <div style={{display:'flex',gap:8}}>
      <DD.Button onClick={model.cancelImport}>Cancel</DD.Button>
      <DD.Button variant="primary" disabled={!!model.importError||!options.groupName.trim()} onClick={model.applyImport}>Import samples</DD.Button>
    </div>
    <span style={{fontSize:12,color:'var(--dd-paper-3)'}}>Esc cancels · files without pitch tokens use the starting note</span>
  </section>;
}
