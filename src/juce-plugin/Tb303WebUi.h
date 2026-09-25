#pragma once

namespace Tb303WebUi
{
inline constexpr auto indexHtml = R"HTML(<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Dandrum TB-303</title>
<style>
:root{font-family:"Arial Narrow",Arial,sans-serif;color:#171717;background:#111}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 50% 15%,#555,#222 45%,#0d0d0d);overflow:auto}.stage{min-height:100vh;display:grid;place-items:center;padding:18px}.machine{width:min(1180px,96vw);padding:22px 25px;border-radius:13px;background:linear-gradient(#d2d0c7,#a5a49b 55%,#8d8d85);border:1px solid #555;box-shadow:0 26px 60px #000b,inset 0 1px #fff,0 0 0 6px #202020}.brand{display:flex;justify-content:space-between;align-items:end;border-bottom:2px solid #222;padding:2px 5px 12px;font-weight:900}.brand .roland{font-size:24px}.brand .model{font-size:29px;color:#d22922}.synth{display:grid;grid-template-columns:repeat(auto-fit,minmax(110px,1fr));gap:14px;padding:20px 4px;border-bottom:2px solid #282828}.control{display:grid;place-items:center;gap:8px}.empty-controls{grid-column:1/-1;padding:35px;text-align:center;font-size:12px;font-weight:900}.label{font-size:10px;font-weight:900;letter-spacing:.7px;text-align:center}.knob-wrap{position:relative;width:98px;height:98px;display:grid;place-items:center}.knob-wrap:before{content:"";position:absolute;inset:3px;border-radius:50%;background:repeating-conic-gradient(from -135deg,#222 0 1deg,transparent 1deg 27deg);clip-path:polygon(0 0,100% 0,100% 80%,50% 50%,0 80%)}.knob{position:relative;width:69px;height:69px;border-radius:50%;background:radial-gradient(circle at 34% 28%,#777,#2b2b2b 35%,#080808 78%);border:2px solid #050505;box-shadow:inset 2px 2px 4px #fff2,inset -4px -5px 7px #000,0 4px 6px #0008;cursor:ns-resize;touch-action:none}.knob i{position:absolute;left:50%;top:5px;width:3px;height:28px;background:#eee;border-radius:2px;transform-origin:50% 30px}.mode button,.step,.fn,.lab-action{border:1px solid #111;border-radius:3px;background:linear-gradient(#555,#151515);color:#eee;box-shadow:inset 0 1px #fff4,0 3px 4px #0007;font-weight:900;cursor:pointer}.mode button.on,.step.on,.fn.on{color:#ff493f;transform:translateY(2px);box-shadow:inset 0 3px 6px #000}.sound-lab{margin:14px 4px;padding:12px 14px;border:2px solid #30312d;border-radius:5px;background:#151916;color:#e5e5dd;box-shadow:inset 0 0 20px #0008}.lab-head{display:grid;grid-template-columns:1fr auto auto;align-items:center;gap:12px}.lab-title{font-size:13px;font-weight:900;letter-spacing:1.2px}.lab-title small{display:block;margin-top:3px;color:#9da39a;font:10px Arial,sans-serif;letter-spacing:.2px}.lab-status{min-width:88px;padding:6px 9px;border:1px solid #475047;border-radius:3px;color:#ffb142;background:#080b09;font:700 11px monospace;text-align:center}.lab-status[data-state="ready"]{color:#72e5a1}.lab-status[data-state="error"]{color:#ff786f}.lab-action{padding:8px 12px;font-size:10px}.lab-action:disabled{cursor:wait;opacity:.55}.lab-body{display:grid;grid-template-columns:minmax(0,1fr) 235px;gap:13px;margin-top:10px}.plot-wrap{position:relative;min-height:174px;border:1px solid #394139;background:#080b09}.plot-wrap canvas{display:block;width:100%;height:174px}.plot-legend{position:absolute;top:6px;right:8px;display:flex;gap:10px;font:9px monospace}.plot-legend span:before{content:"";display:inline-block;width:11px;height:2px;margin-right:4px;vertical-align:middle;background:var(--colour)}.lab-readout{display:grid;align-content:start;gap:8px}.lab-meta{min-height:44px;color:#c2c8c0;font:11px/1.45 monospace}.lab-readout audio{width:100%;height:34px}.lab-note{color:#8f968d;font:9px/1.4 Arial,sans-serif}.middle{display:grid;grid-template-columns:210px 1fr;border-top:2px solid #282828;border-bottom:2px solid #282828}.left{padding:18px;border-right:2px solid #282828}.mode{display:grid;gap:10px}.mode button{height:38px}.status{margin-top:16px;background:#171c18;border:4px solid #30312d;color:#ff543e;font:700 20px monospace;padding:8px;text-align:center;text-shadow:0 0 7px #f31}.seq{padding:18px}.steps{display:grid;grid-template-columns:repeat(8,1fr);gap:7px}.step{height:43px}.step span{display:inline-block;width:8px;height:8px;border-radius:50%;background:#421714;margin-right:6px}.step.on span{background:#ff3b2e;box-shadow:0 0 8px #ff3b2e}.functions{display:grid;grid-template-columns:repeat(6,1fr);gap:7px;margin-top:12px}.fn{height:36px;font-size:9px}.keys{display:grid;grid-template-columns:repeat(19,1fr);gap:4px;padding-top:17px}.key{height:96px;border:1px solid #111;border-radius:2px 2px 5px 5px;background:linear-gradient(90deg,#efeee7,#aaa89f);font-weight:900;cursor:pointer}.key.black{height:70px;color:#eee;background:linear-gradient(90deg,#555,#111)}.key.on{transform:translateY(3px);box-shadow:inset 0 4px 9px #0008}.footer{display:flex;justify-content:space-between;padding-top:12px;font-size:9px;font-weight:900}.error{position:fixed;left:24px;right:24px;bottom:8px;color:#ff786f;font-size:11px;text-align:center;pointer-events:none}@media(max-width:900px){.machine{padding:15px}.stage{padding:8px}.lab-body{grid-template-columns:1fr}.middle{grid-template-columns:160px 1fr}}
</style>
</head>
<body>
<main class="stage">
  <section class="machine">
    <header class="brand"><span class="roland">ROLAND <small>COMPUTER CONTROLLED</small></span><span>BASS LINE <span class="model">TB-303</span></span></header>
    <section class="synth" id="controls"></section>
    <section class="sound-lab" aria-labelledby="soundLabTitle">
      <div class="lab-head">
        <div class="lab-title" id="soundLabTitle">SOUND LAB <small>Maintained 16-step acid fixture · 120 BPM · four bars</small></div>
        <div class="lab-status" id="soundLabStatus" data-state="idle">IDLE</div>
        <button class="lab-action" id="renderSoundLab" type="button">RENDER + ANALYSE</button>
      </div>
      <div class="lab-body">
        <div class="plot-wrap">
          <canvas id="soundLabPlot" width="900" height="174" role="img" aria-label="RMS, peak and spectral-centroid trajectories"></canvas>
          <div class="plot-legend"><span style="--colour:#6ee7ff">RMS</span><span style="--colour:#829087">PEAK</span><span style="--colour:#ffb142">CENTROID</span></div>
        </div>
        <div class="lab-readout">
          <div class="lab-meta" id="soundLabMeta">Render the maintained fixture to inspect its level and spectral movement.</div>
          <audio id="soundLabAudio" controls preload="none"></audio>
          <div class="lab-note">The plot and audition come from one offline Rust workbench render. This is not the live plugin output.</div>
        </div>
      </div>
    </section>
    <section class="middle"><div class="left"><div class="mode"><button class="on">PATTERN</button><button>TRACK</button></div><div class="status" id="status">01</div></div><div class="seq"><div class="steps" id="steps"></div><div class="functions" id="functions"></div></div></section>
    <section class="keys" id="keys"></section>
    <footer class="footer"><span>DIN SYNC &nbsp; 12V DC</span><span>MADE IN JAPAN &nbsp; TB-303</span><span>POWER &nbsp; OUTPUT</span></footer>
  </section>
  <div class="error" id="error"></div>
</main>
<script>
const native=(name)=>(window.__JUCE__&&window.__JUCE__.backend&&window.__JUCE__.backend.getNativeFunction)?window.__JUCE__.backend.getNativeFunction(name):null;
const showError=e=>document.getElementById('error').textContent=String(e||'');
const handleNativeResult=result=>{if(result)showError(result);else showError('')};
const send=(id,value)=>{const fn=native('setParameter');if(fn)fn(id,value).then(handleNativeResult).catch(showError)};
const clamp=value=>Math.max(0,Math.min(1,Number(value)||0));
const knobModels=new Map();

function knob(parameter){
  const c=document.createElement('div');c.className='control';
  const wrap=document.createElement('div');wrap.className='knob-wrap';
  const k=document.createElement('div');k.className='knob';k.tabIndex=0;
  const indicator=document.createElement('i');k.appendChild(indicator);wrap.appendChild(k);
  const label=document.createElement('div');label.className='label';label.textContent=parameter.name||parameter.id;c.append(wrap,label);
  let v=clamp(parameter.value);const draw=()=>indicator.style.transform=`rotate(${-135+v*270}deg)`;const setValue=value=>{v=clamp(value);draw()};draw();
  k.onpointerdown=e=>{k.setPointerCapture(e.pointerId);const sy=e.clientY,sv=v;k.onpointermove=m=>{setValue(sv+(sy-m.clientY)/170);send(parameter.id,v)};const end=()=>{k.onpointermove=null;k.onpointerup=null;k.onpointercancel=null};k.onpointerup=end;k.onpointercancel=end};
  return{element:c,setValue};
}

const controls=document.getElementById('controls');
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

const labStatus=document.getElementById('soundLabStatus');
const labButton=document.getElementById('renderSoundLab');
const labMeta=document.getElementById('soundLabMeta');
const labAudio=document.getElementById('soundLabAudio');
const labCanvas=document.getElementById('soundLabPlot');

function plotSoundLab(frames){
  const ctx=labCanvas.getContext('2d');const w=labCanvas.width,h=labCanvas.height,pad={l:38,r:42,t:22,b:24};
  ctx.clearRect(0,0,w,h);ctx.fillStyle='#080b09';ctx.fillRect(0,0,w,h);
  ctx.strokeStyle='#273027';ctx.lineWidth=1;ctx.fillStyle='#7e887e';ctx.font='9px monospace';
  for(let row=0;row<=4;row++){const y=pad.t+(h-pad.t-pad.b)*row/4;ctx.beginPath();ctx.moveTo(pad.l,y);ctx.lineTo(w-pad.r,y);ctx.stroke()}
  if(!frames||frames.length===0){ctx.fillText('NO ANALYSIS FRAMES',pad.l+8,pad.t+20);return}
  const duration=Math.max(frames[frames.length-1].time_seconds||0,0.001);
  const maxPeak=Math.max(0.01,...frames.map(frame=>Number(frame.peak)||0));
  const x=time=>pad.l+(w-pad.l-pad.r)*time/duration;
  const yLevel=value=>h-pad.b-(h-pad.t-pad.b)*clamp(value/maxPeak);
  const minHz=30,maxHz=20000,logMin=Math.log10(minHz),logRange=Math.log10(maxHz)-logMin;
  const yCentroid=value=>h-pad.b-(h-pad.t-pad.b)*clamp((Math.log10(Math.max(minHz,value))-logMin)/logRange);
  const drawSeries=(valueFor,colour,width)=>{ctx.beginPath();let active=false;for(const frame of frames){const value=valueFor(frame);if(value===null||value===undefined||!Number.isFinite(Number(value))){active=false;continue}const px=x(frame.time_seconds),py=Number(value);if(active)ctx.lineTo(px,py);else{ctx.moveTo(px,py);active=true}}ctx.strokeStyle=colour;ctx.lineWidth=width;ctx.stroke()};
  drawSeries(frame=>yLevel(frame.peak),'#829087',1);
  drawSeries(frame=>yLevel(frame.rms),'#6ee7ff',1.6);
  drawSeries(frame=>frame.spectral_centroid_hz==null?null:yCentroid(frame.spectral_centroid_hz),'#ffb142',1.8);
  ctx.fillStyle='#7e887e';ctx.fillText('LEVEL',4,pad.t+4);ctx.fillText('20k',w-pad.r+7,pad.t+4);ctx.fillText('30 Hz',w-pad.r+4,h-pad.b+3);ctx.fillText('0s',pad.l-4,h-6);ctx.fillText(`${duration.toFixed(1)}s`,w-pad.r-22,h-6);
}

function renderSoundLabState(report){
  report=report||{state:'idle'};const state=report.state||'idle';
  labStatus.dataset.state=state;labStatus.textContent=state.toUpperCase();labButton.disabled=state==='rendering';labButton.textContent=state==='error'?'RETRY RENDER':'RENDER + ANALYSE';
  if(state==='rendering'){
    labMeta.textContent='Rendering the fixture and calculating overlapping Hann-window metrics…';showError('');return;
  }
  if(state==='error'){
    labMeta.textContent='Render failed. Correct the fixture or environment and retry.';showError(report.error||'Sound Lab render failed');return;
  }
  if(state==='ready'){
    const frames=report.metrics||[];plotSoundLab(frames);
    labMeta.textContent=`${Number(report.sample_rate_hz).toLocaleString()} Hz · ${Number(report.duration_seconds).toFixed(2)} s · ${frames.length.toLocaleString()} analysis frames`;
    const generation=String(report.generation);if(labAudio.dataset.generation!==generation){labAudio.dataset.generation=generation;labAudio.src=report.audio_url;labAudio.load()}
    showError('');return;
  }
  plotSoundLab([]);labMeta.textContent='Render the maintained fixture to inspect its level and spectral movement.';
}

labButton.onclick=()=>{const fn=native('renderSoundLab');if(!fn){showError('Sound Lab native bridge is unavailable');return}renderSoundLabState({state:'rendering'});fn().then(handleNativeResult).catch(showError)};

const steps=document.getElementById('steps');for(let n=0;n<16;n++){const b=document.createElement('button');b.className='step'+(n===0?' on':'');b.innerHTML=`<span></span>${n+1}`;b.onclick=()=>{document.querySelectorAll('.step').forEach(x=>x.classList.remove('on'));b.classList.add('on');document.getElementById('status').textContent=String(n+1).padStart(2,'0')};steps.appendChild(b)}
for(const name of ['TRANSPOSE DOWN','TRANSPOSE UP','ACCENT','SLIDE','REST','TIE']){const b=document.createElement('button');b.className='fn';b.textContent=name;b.onclick=()=>b.classList.toggle('on');document.getElementById('functions').appendChild(b)}
const notes=['C','C♯','D','D♯','E','F','F♯','G','G♯','A','A♯','B','C','C♯','D','D♯','E','F','F♯'];notes.forEach((note,n)=>{const b=document.createElement('button');b.className='key'+(note.includes('♯')?' black':'');b.textContent=note;b.onpointerdown=()=>{b.classList.add('on');const fn=native('noteOn');if(fn)fn(48+n,0.9).then(handleNativeResult).catch(showError)};const release=()=>{if(!b.classList.contains('on'))return;b.classList.remove('on');const fn=native('noteOff');if(fn)fn(48+n).then(handleNativeResult).catch(showError)};b.onpointerup=b.onpointerleave=b.onpointercancel=release;document.getElementById('keys').appendChild(b)});

const get=native('getParameters');if(get)get().then(renderParameters).catch(showError);else renderParameters([]);
const getSoundLab=native('getSoundLabAnalysis');if(getSoundLab)getSoundLab().then(renderSoundLabState).catch(showError);else renderSoundLabState({state:'idle'});
if(window.__JUCE__&&window.__JUCE__.backend){window.__JUCE__.backend.addEventListener('parameterValuesChanged',updateParameterValues);window.__JUCE__.backend.addEventListener('soundLabAnalysisChanged',renderSoundLabState)}
</script>
</body>
</html>)HTML";
}
