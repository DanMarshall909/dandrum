import React from 'react';
import * as DD from './design-system/index.jsx';
export function ProcessorStack({model,layers,title,subtitle}){
  const drag=React.useRef(null);
  return <section className="processor-stack"><div style={{display:'flex',gap:10,alignItems:'baseline',font:'600 12px var(--font-ui)',color:'var(--dd-paper-2)'}}><span>{title}</span><span style={{font:'600 11px var(--font-ui)'}}>{subtitle}</span></div>
    {layers.map(chain=><div key={chain.id} data-chain={chain.id} onPointerDownCapture={event=>{
      const button=event.target.closest('[role="button"][aria-label]'),module=chain.modules.find(m=>button?.dataset.moduleId===m.id);
      if(module){button.draggable=true;model.selectProcessor(module.id);}
    }} onDragStart={event=>{const id=event.target.closest('[data-module-id]')?.dataset.moduleId,module=chain.modules.find(m=>m.id===id);if(!module)return;drag.current=module.id;event.dataTransfer.setData('text/plain',module.id);}}
      onDragOver={event=>{if(drag.current){event.preventDefault();event.dataTransfer.dropEffect='move';}}}
      onDrop={event=>{if(!drag.current)return;event.preventDefault();event.stopPropagation();const button=event.target.closest('[role="button"][aria-label]'),index=chain.modules.findIndex(m=>m.id===button?.dataset.moduleId);model.moveProcessor(drag.current,chain.id,index<0?chain.modules.length:index);drag.current=null;}}
      onDragEnd={()=>drag.current=null}>
      <DD.LayerStack layers={[chain]} selectedId={model.selectedChain} onSelect={model.selectChain} onChange={model.changeLayers} onAddModule={model.addProcessor} outputs={model.outputs} title="" countLabel=""/>
    </div>)}
  </section>;
}
