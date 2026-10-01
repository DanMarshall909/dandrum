#pragma once

namespace SharedInstrumentUi
{
inline constexpr auto script = R"JS(
const native=name=>(window.__JUCE__&&window.__JUCE__.backend&&window.__JUCE__.backend.getNativeFunction)
  ?window.__JUCE__.backend.getNativeFunction(name):null;
const showError=error=>document.getElementById('error').textContent=String(error||'');
const handleNativeResult=result=>showError(result||'');
const send=(id,value)=>{const fn=native('setParameter');if(fn)fn(id,value).then(handleNativeResult).catch(showError)};
const clamp=value=>Math.max(0,Math.min(1,Number(value)||0));
const knobModels=new Map();
const controls=document.getElementById('controls');

function knob(parameter){
  const element=document.createElement('div');element.className='control';
  const wrap=document.createElement('div');wrap.className='knob-wrap';
  const face=document.createElement('div');face.className='knob';face.tabIndex=0;
  const indicator=document.createElement('i');face.appendChild(indicator);wrap.appendChild(face);
  const label=document.createElement('div');label.className='label';label.textContent=parameter.name||parameter.id;
  element.append(wrap,label);
  let value=clamp(parameter.value);
  const draw=()=>indicator.style.transform=`rotate(${-135+value*270}deg)`;
  const setValue=next=>{value=clamp(next);draw()};draw();
  face.onpointerdown=event=>{
    face.setPointerCapture(event.pointerId);
    const startY=event.clientY,startValue=value;
    face.onpointermove=move=>{setValue(startValue+(startY-move.clientY)/170);send(parameter.id,value)};
    const end=()=>{face.onpointermove=null;face.onpointerup=null;face.onpointercancel=null};
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

const get=native('getParameters');if(get)get().then(renderParameters).catch(showError);else renderParameters([]);
if(window.__JUCE__&&window.__JUCE__.backend)
  window.__JUCE__.backend.addEventListener('parameterValuesChanged',updateParameterValues);
)JS";
}
