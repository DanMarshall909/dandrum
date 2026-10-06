import React from 'react';
import * as DD from './design-system/index.jsx';
export function AssetBrowser({model}){return <>
        <div style={{"height": "30px","flex": "none","display": "flex","alignItems": "center","gap": "6px","padding": "0 8px 0 12px","background": "var(--dd-ink-3)","borderBottom": "1px solid var(--dd-line-1)"}}>
          <span style={{"font": "700 12px var(--font-ui)","letterSpacing": ".1em","textTransform": "uppercase","color": "var(--dd-paper-2)"}}>Samples</span>
          {model.ann && <><span style={{"font": "500 11px/16px var(--font-value)","color": "var(--dd-warn)","background": "var(--dd-warn-wash)","border": "1px dashed var(--dd-warn)","borderRadius": "2px","padding": "0 5px","whiteSpace": "nowrap"}}>Asset service</span></>}
          <div style={{"flex": "1"}}></div>
          <DD.IconButton icon="close" label="Close browser" size="sm" variant="ghost" onClick={model.toggleBrowser}></DD.IconButton>
        </div>
        <div style={{"padding": "8px","display": "flex","flexDirection": "column","gap": "6px","borderBottom": "1px solid var(--dd-line-1)"}}>
          <div style={{"height": "26px","display": "flex","alignItems": "center","gap": "6px","padding": "0 8px","background": "var(--dd-ink-0)","border": "1px solid var(--dd-line-2)","borderRadius": "2px","font": "500 13px var(--font-ui)","color": "var(--dd-paper-3)"}}><input aria-label="Search samples" value={model.browserQuery} onChange={e=>model.setBrowserQuery(e.target.value)} placeholder="Search samples" style={{width:'100%',minWidth:0,border:0,background:'transparent',color:'var(--dd-paper-1)',fontSize:13}}/></div>
          <DD.SegmentedControl compact={true} fullWidth={true} value={model.browserAssetFilter} onChange={model.setBrowserAssetFilter} options={model.browserFilter}></DD.SegmentedControl>
        </div>
        <div style={{"flex": "1","minHeight": "0","overflow": "auto","padding": "4px 0"}}>
          {(model.assets || []).map((a,index) => <React.Fragment key={a.id ?? a.key ?? index}>
            {a.head && <>
              <div style={{"padding": "8px 12px 4px","font": "600 11px var(--font-ui)","letterSpacing": ".06em","textTransform": "uppercase","color": "var(--dd-paper-3)"}}>{a.label}</div>
            </>}
            {a.row && <>
              <div data-ctx={"src|"+a.id} draggable={true} onDragStart={a.drag} onDragEnd={model.endSrcDrag} style={{"padding": "0 4px"}}>
                <DD.ListRow icon={a.icon} label={a.label} value={a.dur} selected={a.sel} onClick={a.play}></DD.ListRow>
              </div>
            </>}
          </React.Fragment>)}
        </div>
        <div style={{"padding": "8px 12px","borderTop": "1px solid var(--dd-line-1)","font": "500 12px/1.35 var(--font-ui)","color": "var(--dd-paper-3)"}}>Click to preview. Drag onto the waveform to replace, onto the key map to add a zone.</div>
 </>; }
