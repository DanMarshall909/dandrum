import React from 'react';
import * as DD from './design-system/index.jsx';
import {ControlMenu} from './GenericMenu.jsx';
import {DragLayer} from './DragLayer.jsx';
import {StatusBar} from './StatusBar.jsx';
import {Inspector} from './Inspector.jsx';
import {Workspace} from './Workspace.jsx';
import {AssetSidebar} from './AssetSidebar.jsx';
import {WorkspaceTabs} from './WorkspaceTabs.jsx';
import {MacroStrip} from './MacroStrip.jsx';
import {AppHeader} from './AppHeader.jsx';

export function TriggerFrame({model}) { return (<div ref={model.winRef} onDragOver={model.onFileDrag} onDrop={model.onFileDrop} onContextMenu={model.onRootCtx} onMouseDown={model.onRootDown} onAuxClick={model.onAux} data-screen-label={model.stateLabel} style={{"boxSizing": "border-box","width": (model.W) + "px","height": (model.H) + "px","display": "flex","flexDirection": "column","background": "var(--dd-ink-1)","border": "1px solid var(--dd-window-border)","boxShadow": "var(--dd-window-shadow)","overflow": "hidden","position": "relative"}} data-testid="trigger-editor">

  <AppHeader model={model}/>

  <MacroStrip model={model}/>

  {model.notPerf && <>
  <WorkspaceTabs model={model}/>

  {model.banner && <>
    <div style={{"padding": "4px 4px 0","flex": "none","display": "flex","flexDirection": "column","gap": "4px"}}>
      {model.missing && <>
        <DD.StatusMessage kind="error" compact={true} title={model.assetName+" · sample unavailable"}>{model.missingMessage}</DD.StatusMessage>
      </>}
      {model.unsup && <>
        <DD.StatusMessage kind="warn" compact={true} title="File was not loaded">{model.unsupportedMessage??'AAC audio is not supported. Use WAV, AIFF, FLAC, MP3 or OGG. The current sample is unchanged.'}</DD.StatusMessage>
      </>}
    </div>
  </>}

  <div style={{"flex": "1","minHeight": "0","display": "grid","gridTemplateColumns": model.gridCols,"gap": "4px","padding": "4px"}}>

    <AssetSidebar model={model}/>

    <Workspace model={model}/>

    <Inspector model={model}/>
  </div>


  {model.isMin&&model.rail&&<div role="region" aria-label="Sidebar drawer" style={{position:'absolute',top:156,bottom:30,left:42,width:208,display:'flex',zIndex:15,boxShadow:'var(--shadow-float)'}}><AssetSidebar model={{...model,isMin:false,showTree:model.rail==='tree',showBrowser:model.rail==='browser'}}/></div>}
  <StatusBar model={model}/>
  </>}
  <DragLayer dragging={model.dragging} hint={model.hint}/>
  {model.logDrawer}
  {model.kmenuOpen && <>
    <div onPointerDown={model.closeK} onContextMenu={model.closeKCtx} style={{"position": "fixed","inset": "0","zIndex": "30"}}></div>
    {(model.kmenus || []).map((m,index) => <React.Fragment key={m.id ?? m.key ?? index}>
      <div style={{"position": "absolute","left": (m.x) + "px","top": (m.y) + "px","zIndex": "31",maxHeight:model.H-m.y-4,overflowY:'auto'}}>
        <ControlMenu items={m.items} title={m.title} width={260} onClose={model.closeK}></ControlMenu>
      </div>
    </React.Fragment>)}
  </>}
</div>); }
