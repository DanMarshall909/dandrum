import React from 'react';
import * as DD from './design-system/index.jsx';
import {EmptyPage} from './pages/EmptyPage.jsx';
import {DropPage} from './pages/DropPage.jsx';
import {SamplePage} from './pages/SamplePage.jsx';
import {SlicesPage} from './pages/SlicesPage.jsx';
import {MappingPage} from './pages/MappingPage.jsx';
import {LayersPage} from './pages/LayersPage.jsx';
import {VoicePage} from './pages/VoicePage.jsx';
import {ModulationPage} from './pages/ModulationPage.jsx';
import {RoutingPage} from './pages/RoutingPage.jsx';
import {EffectsPage} from './pages/EffectsPage.jsx';

export function Workspace({model}) {
  return (<main onContextMenu={model.onKnobCtx} onDragOver={model.onKnobOver} onDrop={model.onKnobDrop} onDragLeave={model.onKnobLeave} style={{"minWidth": "0","minHeight": "0","overflow": "auto","display": "flex","flexDirection": "column","gap": "4px"}}>


      <EmptyPage model={model}/>


      <DropPage model={model}/>


      <SamplePage model={model}/>


      <SlicesPage model={model}/>


      <MappingPage model={model}/>


      <LayersPage model={model}/>


      <VoicePage model={model}/>


      <ModulationPage model={model}/>


      <RoutingPage model={model}/>


      <EffectsPage model={model}/>

      {model.isExp && <>
        <section style={{"background": "var(--dd-ink-2)","border": "1px solid var(--dd-line-1)","borderRadius": "6px","padding": "6px 8px"}}>
          <DD.KeyMap zones={model.zones} selectedId={model.zoneSel} onSelect={model.selZone} onNoteOn={model.onKeyOn} onNoteOff={model.onKeyOff} lowNote={24} highNote={96} gridHeight={56} keyboardHeight={44} editable={false} title="Key map · read-only overview"></DD.KeyMap>
        </section>
      </>}
    </main>);
}
