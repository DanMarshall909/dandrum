import React from 'react';
import * as DD from '../design-system/index.jsx';
export function EmptyPage({model}) { return <>{model.isEmpty && <>
        <div style={{"flex": "1","minHeight": "360px","display": "flex","flexDirection": "column","alignItems": "center","justifyContent": "center","gap": "16px","border": "1px dashed var(--dd-line-3)","borderRadius": "6px","background": "var(--dd-ink-0)"}}>
          <DD.EmptyState icon="plus" title="Drop samples here">One file becomes a sound across the keyboard. Several files ask how to map them.</DD.EmptyState>
          <div style={{"display": "flex","gap": "8px"}}>
            <DD.Button variant="primary" icon="folder" onClick={model.browseSamples}>Browse samples…</DD.Button>
            <DD.Button variant="secondary" icon="layers" onClick={model.browseSamples}>Load multiple…</DD.Button>
          </div>
          <span style={{"font": "500 12px var(--font-ui)","color": "var(--dd-paper-3)"}}>WAV · AIFF · FLAC · MP3 · OGG</span>
        </div>
      </>}</>; }
