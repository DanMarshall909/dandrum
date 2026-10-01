#pragma once

namespace SamplerWebUi
{
inline constexpr auto indexHtml = R"HTML(<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Dandrum Drum Sampler</title>
<style>
:root{font-family:Inter,Arial,sans-serif;color:#f2f1e8;background:#0f1518}*{box-sizing:border-box}body{margin:0;background:radial-gradient(circle at 70% 10%,#254047,#0f1518 65%)}main{max-width:980px;margin:auto;padding:40px 28px;display:grid;gap:24px}.topline{border-bottom:3px solid #4de0bf;padding-bottom:20px}.eyebrow{color:#89ffe1;font-size:12px;font-weight:800;letter-spacing:.2em;text-transform:uppercase}h1{margin:8px 0 0;font-size:clamp(40px,8vw,78px);line-height:.95;letter-spacing:-.06em}h1 span{color:#4de0bf}.panel{padding:24px;border:1px solid #3b6263;border-radius:12px;background:#182629;box-shadow:0 18px 48px #0007}.panel h2{margin:0 0 20px;color:#b5d5d0;font-size:12px;letter-spacing:.15em;text-transform:uppercase}.controls{display:grid;grid-template-columns:repeat(auto-fit,minmax(120px,1fr));gap:24px}.control{display:grid;justify-items:center;gap:12px}.knob-wrap{width:92px;height:92px;display:grid;place-items:center;border-radius:50%;background:repeating-conic-gradient(#337e73 0 1deg,transparent 1deg 27deg)}.knob{position:relative;width:70px;height:70px;border-radius:50%;background:radial-gradient(circle at 30% 25%,#526c69,#142225 65%);border:2px solid #091415;box-shadow:0 5px 10px #000a;cursor:ns-resize;touch-action:none}.knob i{position:absolute;left:50%;top:6px;width:3px;height:27px;border-radius:2px;background:#54f7ca;transform-origin:50% 29px}.label{font-size:12px;font-weight:700;text-align:center}.empty-controls{grid-column:1/-1;color:#a6bebb}.drum-pads{display:grid;grid-template-columns:repeat(4,minmax(0,1fr));gap:12px}.key{height:110px;border:1px solid #6adcc0;border-radius:8px;background:linear-gradient(#4a8e83,#285957);color:#f8fff9;font-size:16px;font-weight:800;cursor:pointer;touch-action:none}.key.on{transform:translateY(4px);filter:brightness(1.25)}.hint{color:#b0c4bd;font-size:13px;line-height:1.5}.error{min-height:18px;color:#ff9a79;font-size:12px}@media(max-width:650px){main{padding:22px 14px}.panel{padding:18px}.drum-pads{grid-template-columns:repeat(2,minmax(0,1fr))}.key{height:90px}}
</style>
</head>
<body>
<main>
  <header class="topline"><div class="eyebrow">Dandrum · prepared sample kit</div><h1>DRUM <span>SAMPLER</span></h1></header>
  <section class="panel" aria-labelledby="controlsTitle"><h2 id="controlsTitle">Host-modulatable controls</h2><div class="controls" id="controls"></div></section>
  <section class="panel" aria-labelledby="drumsTitle"><h2 id="drumsTitle">Play the kit</h2><div class="drum-pads" id="keys" data-drum-notes="36:Kick,38:Snare,42:Closed Hat,46:Open Hat"></div></section>
  <div class="hint">Snare velocities 1–63 play the soft hit; 64–127 play the hard alternates. Closed hat cuts the open hat. Drag a control or map a host modulator to its parameter.</div>
  <div class="error" id="error" role="status" aria-live="polite"></div>
</main>
<script src="/shared-instrument-ui.js"></script>
</body>
</html>)HTML";
}
