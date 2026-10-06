import React,{useEffect,useState,useSyncExternalStore} from 'react';
import {MockEngine} from '../engine/mock-engine.mjs';
import {Store} from '../engine/store.mjs';
import {ShellPresenter} from '../shell/ShellPresenter.mjs';
import {stories,states,engineCalls} from './registry.mjs';
import {storyProps} from './story-props.mjs';
import * as DD from '../components/design-system/index.jsx';
class StoryBoundary extends React.Component{state={error:null};static getDerivedStateFromError(error){return {error};}render(){return this.state.error?<div role="alert" data-story-error>{this.state.error.message}</div>:this.props.children;}}
export function CatalogController(){
  const [runtime]=useState(()=>{const engine=new MockEngine({preset:'felt-kit',preparationMs:0}),store=new Store(engine,{diagnostics:engine});return {engine,store,presenter:new ShellPresenter(store)};});
  useEffect(()=>()=>{runtime.store.close();runtime.engine.close();},[runtime]);useSyncExternalStore(runtime.store.subscribe,runtime.store.getSnapshot);
  const [id,setId]=useState('ParameterKnob'),[width,setWidth]=useState(1200),[background,setBackground]=useState('ink'),[query,setQuery]=useState(''),[events,setEvents]=useState([]),[controls,setControls]=useState({state:'default',label:'Filter cutoff',value:.42,checked:true,shape:'adsr'});
  const selected=stories.find(s=>s.id===id)??stories[0],base=runtime.presenter.buildModel(),emit=(name,args)=>setEvents(e=>[...e.slice(-29),{name,args:args.map(arg=>arg?.nativeEvent?{type:arg.type,key:arg.key,button:arg.button}:arg instanceof Event?{type:arg.type}:arg)}]);
  base.pianoPeaks=Array.from(runtime.engine.getPeaks('k60f',600));base.slicePeaks=Array.from(runtime.engine.getPeaks('amen',600));
  const props=storyProps(selected.name,base,controls,emit),Component=selected.component,update=(key,value)=>setControls(c=>({...c,[key]:value}));
  const groups=[...new Set(stories.map(s=>s.group))],describe=value=>typeof value==='function'?'callback':Array.isArray(value)?'Array('+value.length+')':value===null?'null':typeof value==='object'?'Object':String(value);
  return <div className="catalog"><aside className="catalog-nav"><h1>Trigger catalog</h1><a href="/">Open shell</a><input aria-label="Find component" value={query} onChange={e=>setQuery(e.target.value)} placeholder="Find component"/>
    {groups.map(group=><section key={group}><h2>{group}</h2>{stories.filter(s=>s.group===group&&s.name.toLowerCase().includes(query.toLowerCase())).map(s=><button key={s.id} aria-pressed={id===s.id} onClick={()=>{setId(s.id);setEvents([]);}}>{s.name}</button>)}</section>)}
  </aside><main className="catalog-main"><header><h1>{selected.name}</h1><span>{selected.path}</span><div className="catalog-toolbar"><DD.SegmentedControl label="Canvas width" value={width} options={[820,1200,1600].map(id=>({id,label:String(id)}))} onChange={setWidth}/><DD.SegmentedControl label="Background" value={background} options={['ink','paper','checker'].map(id=>({id,label:id}))} onChange={setBackground}/></div></header>
    <section aria-label="Story controls" className="catalog-controls"><label>State <select value={controls.state} onChange={e=>update('state',e.target.value)}>{states.map(state=><option key={state}>{state}</option>)}</select></label><label>Label <input value={controls.label} onChange={e=>update('label',e.target.value)}/></label><label>Value <input type="range" min="0" max="1" step=".01" value={controls.value} onChange={e=>update('value',Number(e.target.value))}/><output>{controls.value}</output></label><label>Checked <input type="checkbox" checked={controls.checked} onChange={e=>update('checked',e.target.checked)}/></label><label>Shape <select value={controls.shape} onChange={e=>update('shape',e.target.value)}>{['adsr','ahdsr','multi','sine','tri','saw','sq','sh'].map(v=><option key={v}>{v}</option>)}</select></label></section>
    <div className="catalog-scroll"><div data-story-canvas data-testid="trigger-editor" className={'catalog-preview '+background} style={{width,padding:16,position:'relative',boxSizing:'border-box'}}><div style={{width:controls.state==='minimum width'?280:'100%',minHeight:200}} inert={controls.state==='disabled'?true:undefined}><StoryBoundary key={selected.id+'|'+controls.state}><Component {...props}/></StoryBoundary></div></div></div>
    <section><h2>Props</h2><table><thead><tr><th>Prop</th><th>Type</th><th>Value</th></tr></thead><tbody>{Object.entries(props).map(([key,value])=><tr key={key}><td>{key}</td><td>{typeof value}</td><td>{describe(value)}</td></tr>)}</tbody></table></section>
    <section><h2>Engine callbacks</h2><p>{engineCalls[selected.name]??'Presentation callbacks supplied by the parent; no engine imports.'}</p><div role="log" aria-label="Story callback log">{events.map((e,i)=><div key={i}>{e.name} {JSON.stringify(e.args)}</div>)}</div></section>
  </main></div>;
}
