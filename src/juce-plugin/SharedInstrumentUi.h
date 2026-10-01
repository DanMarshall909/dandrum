#pragma once

namespace SharedInstrumentUi
{
inline constexpr auto script = R"JS(
const native=name=>(window.__JUCE__&&window.__JUCE__.backend&&window.__JUCE__.backend.getNativeFunction)
  ?window.__JUCE__.backend.getNativeFunction(name):null;
const showError=error=>document.getElementById('error').textContent=String(error||'');
let currentGeneration=0,lastAdmittedSequence=0,pendingWrites=0;
const handleNativeResult=result=>{
  if(typeof result==='string'){showError(result);requestState();return}
  if(result&&result.status==='accepted'&&result.generation===currentGeneration)
    lastAdmittedSequence=Math.max(lastAdmittedSequence,Number(result.sequence)||0);
  showError('');
};
const submit=(command,...args)=>{
  const fn=native(command);if(!fn)return;
  const write=command==='setParameter';if(write)pendingWrites++;
  fn(...args).then(result=>{if(write)pendingWrites=Math.max(0,pendingWrites-1);handleNativeResult(result)})
    .catch(error=>{if(write)pendingWrites=Math.max(0,pendingWrites-1);showError(error);requestState()});
};
const send=(id,value)=>submit('setParameter',id,value,currentGeneration);
const gesture=(command,id)=>submit(command,id,currentGeneration);
const clamp=value=>Math.max(0,Math.min(1,Number(value)||0));
const knobModels=new Map();
const controls=document.getElementById('controls');

function knob(parameter){
  const element=document.createElement('div');element.className='control';
  const wrap=document.createElement('div');wrap.className='knob-wrap';
  const face=document.createElement('div');face.className='knob';face.tabIndex=0;
  const indicator=document.createElement('i');face.appendChild(indicator);wrap.appendChild(face);
  const label=document.createElement('div');label.className='label';label.textContent=parameter.name||parameter.id;
  const entry=document.createElement('input');entry.className='value-entry';entry.type='number';
  entry.min='0';entry.max='1';entry.step='0.01';entry.ariaLabel=`${label.textContent} normalized value`;
  element.append(wrap,label,entry);
  let value=clamp(parameter.value);
  const draw=()=>{indicator.style.transform=`rotate(${-135+value*270}deg)`;entry.value=value.toFixed(3)};
  let dragging=false;
  const setLocal=next=>{value=clamp(next);draw()};
  const setValue=next=>{if(!dragging)setLocal(next)};draw();
  face.onkeydown=event=>{
    let next=value;
    switch(event.key){
      case'ArrowUp':case'ArrowRight':next+=0.01;break;
      case'ArrowDown':case'ArrowLeft':next-=0.01;break;
      case'PageUp':next+=0.1;break;
      case'PageDown':next-=0.1;break;
      case'Home':next=0;break;
      case'End':next=1;break;
      default:return;
    }
    event.preventDefault();setLocal(next);send(parameter.id,value);
  };
  entry.onchange=()=>{
    const typed=Number(entry.value);
    if(entry.value.trim()===''||!Number.isFinite(typed)||typed<0||typed>1){draw();return}
    setLocal(typed);send(parameter.id,value);
  };
  face.onpointerdown=event=>{
    face.setPointerCapture(event.pointerId);
    dragging=true;
    gesture('beginGesture',parameter.id);
    const startY=event.clientY,startValue=value;
    face.onpointermove=move=>{setLocal(startValue+(startY-move.clientY)/170);send(parameter.id,value)};
    const end=()=>{dragging=false;face.onpointermove=null;face.onpointerup=null;face.onpointercancel=null;gesture('endGesture',parameter.id)};
    face.onpointerup=end;face.onpointercancel=end;
  };
  return{element,setValue};
}

function renderParameters(parameters){
  controls.replaceChildren();knobModels.clear();
  for(const parameter of parameters||[]){const model=knob(parameter);knobModels.set(parameter.id,model);controls.appendChild(model.element)}
  if(knobModels.size===0){const empty=document.createElement('div');empty.className='empty-controls';empty.textContent='NO PUBLIC PARAMETERS';controls.appendChild(empty)}
}

function updateParameterValues(parameters){
  const incoming=parameters||[];
  if(incoming.length!==knobModels.size||incoming.some(parameter=>!knobModels.has(parameter.id))){renderParameters(incoming);return}
  for(const parameter of incoming)knobModels.get(parameter.id).setValue(parameter.value);
}

function applyState(state){
  if(!state||!Array.isArray(state.parameters)||!Number.isInteger(state.generation)
     ||!Number.isInteger(state.sequence))return;
  if(state.generation<currentGeneration)return;
  if(state.generation>currentGeneration){
    currentGeneration=state.generation;lastAdmittedSequence=state.sequence;pendingWrites=0;
    renderParameters(state.parameters);return;
  }
  if(state.sequence<lastAdmittedSequence||pendingWrites>0)return;
  updateParameterValues(state.parameters);
}

function requestState(){const get=native('getParameterState');if(get)get().then(applyState).catch(showError)}

const notes=['C','C♯','D','D♯','E','F','F♯','G','G♯','A','A♯','B','C','C♯','D','D♯','E','F','F♯'];
const keys=document.getElementById('keys');
const drumNotes=(keys.dataset.drumNotes||'').split(',').filter(Boolean).map(entry=>{
  const separator=entry.indexOf(':');
  return{number:Number(entry.slice(0,separator)),label:entry.slice(separator+1)};
}).filter(entry=>Number.isInteger(entry.number)&&entry.number>=0&&entry.number<=127&&entry.label);
const playableNotes=drumNotes.length?drumNotes:notes.map((label,index)=>({number:48+index,label}));
playableNotes.forEach(({number,label})=>{
  const key=document.createElement('button');key.className='key'+(!drumNotes.length&&label.includes('♯')?' black':'');key.textContent=label;
  key.onpointerdown=()=>{key.classList.add('on');const fn=native('noteOn');if(fn)fn(number,0.9).then(handleNativeResult).catch(showError)};
  const release=()=>{if(!key.classList.contains('on'))return;key.classList.remove('on');const fn=native('noteOff');if(fn)fn(number).then(handleNativeResult).catch(showError)};
  key.onpointerup=key.onpointerleave=key.onpointercancel=release;
  keys.appendChild(key);
});

requestState();
if(window.__JUCE__&&window.__JUCE__.backend)
  window.__JUCE__.backend.addEventListener('parameterStateChanged',applyState);
)JS";
}
