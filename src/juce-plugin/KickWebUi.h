#pragma once

namespace KickWebUi
{
inline constexpr auto indexHtml = R"HTML(<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Dandrum 808 Kick</title>
<style>
:root{font-family:Inter,Arial,sans-serif;color:#f4eee2;background:#100e0c}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 70% 15%,#3b2619,#100e0c 60%)}main{max-width:1080px;min-height:100vh;margin:auto;padding:42px 30px;display:grid;align-content:center;gap:26px}.topline{display:flex;justify-content:space-between;align-items:start;gap:20px;border-bottom:4px solid #e87929;padding-bottom:20px}.eyebrow{color:#f49a59;font-size:12px;font-weight:800;letter-spacing:.25em;text-transform:uppercase}h1{margin:8px 0 0;font-size:clamp(44px,9vw,92px);line-height:.9;letter-spacing:-.07em}h1 span{color:#f47b2c}.badge{border:1px solid #a45523;padding:11px 15px;color:#ffa86a;font-size:11px;font-weight:800;letter-spacing:.1em}.panel{padding:26px;border:1px solid #62402c;border-radius:12px;background:linear-gradient(145deg,#2b221c,#181512);box-shadow:0 20px 60px #0008,inset 0 1px #ffffff12}.panel h2{margin:0 0 24px;color:#c9b8a9;font-size:12px;letter-spacing:.2em;text-transform:uppercase}.controls{display:grid;grid-template-columns:repeat(auto-fit,minmax(120px,1fr));gap:28px}.control{display:grid;justify-items:center;gap:12px}.knob-wrap{width:94px;height:94px;display:grid;place-items:center;border-radius:50%;background:repeating-conic-gradient(#a35e30 0 1deg,transparent 1deg 27deg);box-shadow:0 0 0 7px #181512}.knob{position:relative;width:70px;height:70px;border-radius:50%;background:radial-gradient(circle at 30% 25%,#5c5149,#201b18 65%);border:2px solid #0b0908;box-shadow:0 5px 10px #000a,inset 0 1px #ffffff38;cursor:ns-resize;touch-action:none}.knob i{position:absolute;left:50%;top:6px;width:3px;height:27px;border-radius:2px;background:#ff8d43;transform-origin:50% 29px}.label{font-size:11px;font-weight:700;text-align:center}.empty-controls{grid-column:1/-1;color:#a8998b}.keyboard{display:grid;grid-template-columns:repeat(19,1fr);gap:5px}.key{height:95px;border:1px solid #a56b45;border-radius:5px;background:linear-gradient(#d8c8b3,#a58b74);color:#241811;font-weight:800;cursor:pointer}.key.black{height:70px;background:linear-gradient(#504238,#201c19);color:#f1dfca}.key.on{transform:translateY(4px);filter:brightness(1.2)}.hint{color:#aa9481;font-size:12px}.error{min-height:18px;color:#ff9a79;font-size:12px}/*sound-lab-style*/@media(max-width:650px){main{padding:24px 14px}.topline{display:block}.badge{display:inline-block;margin-top:18px}.panel{padding:20px}.keyboard{gap:2px}.key{font-size:9px}}
</style>
</head>
<body>
<main>
  <header class="topline"><div><div class="eyebrow">Dandrum · synthetic drum voice</div><h1>808 <span>KICK</span></h1></div><div class="badge">ANALOG BASS DRUM</div></header>
  <section class="panel" aria-labelledby="controlsTitle"><h2 id="controlsTitle">Instrument controls</h2><div class="controls" id="controls"></div></section>
  <!--sound-lab-panel-->
  <section class="panel" aria-labelledby="keyboardTitle"><h2 id="keyboardTitle">Play the voice</h2><div class="keyboard" id="keys"></div></section>
  <div class="hint">Drag controls vertically. Press and release a key to trigger the drum voice.</div>
  <div class="error" id="error" role="status" aria-live="polite"></div>
</main>
<script src="/shared-instrument-ui.js"></script>
<!--sound-lab-script-->
</body>
</html>)HTML";
}
