import React from 'react';
import {createRoot} from 'react-dom/client';
import {TriggerController} from './TriggerController.jsx';
import {MockEngine} from './engine/mock-engine.mjs';
import {NativeEngine} from './engine/native-engine.mjs';
import {Store} from './engine/store.mjs';
import './components/design-system/styles.css';
import './components/source-hover.css';
import './styles.css';
const root=createRoot(document.getElementById('root'));
let engine,store,disposed=false;
async function launch(){
  const native=!!window.__JUCE__?.backend?.getNativeFunction;
  root.render(<div role="status">{native?'Connecting to native sampler…':'Opening silent sampler preview…'}</div>);
  try{
    engine=native?new NativeEngine():new MockEngine();
    if(native)await engine.initialize();
    if(disposed){engine.close();return;}
    store=new Store(engine,{diagnostics:native?null:engine});
    if(native){const patch=engine.getPatch();store.select({st:patch.rules.length?'sample':'empty',zoneSel:patch.rules[0]?.id,snapZero:false});}
    root.render(<div className="preview">
      <div role="status" data-backend={native?'native':'mock'} style={{padding:'6px 12px',font:'12px sans-serif',background:'#181815',color:'#c8c7bd'}}>
        {native?'Audio engine connected · Sample playback and editing · Effects, modulation and analysis unavailable':'Silent browser preview'}
      </div>
      <TriggerController store={store}/>
    </div>);
  }catch(error){engine?.close();root.render(<div role="alert">Native sampler failed: {error.message}</div>);}
}
void launch();
window.addEventListener('pagehide',()=>{store?.close();engine?.close();});
if(import.meta.hot)import.meta.hot.dispose(()=>{disposed=true;root.unmount();store?.close();engine?.close();});
