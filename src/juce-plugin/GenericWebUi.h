#pragma once

namespace GenericWebUi
{
inline constexpr auto indexHtml = R"HTML(<!doctype html>
<html lang="en">
<head>
<meta charset="utf-8">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>Dandrum Instrument</title>
<style>
:root{font-family:Arial,sans-serif;color:#e8eee9;background:#171a18}*{box-sizing:border-box}
body{margin:0}main{max-width:960px;margin:auto;padding:32px;display:grid;gap:24px}
h1{font-size:30px;margin:0}.panel{padding:20px;border:1px solid #45534a;border-radius:8px;background:#212823}
.controls{display:flex;flex-wrap:wrap;gap:18px}.control{display:grid;justify-items:center;gap:8px;min-width:110px}
.knob{width:62px;height:62px;border-radius:50%;border:2px solid #779983;background:#35463b;cursor:ns-resize}
.knob i{display:block;width:3px;height:24px;margin:5px auto;background:#d6f4dd}.label{font-size:12px}
.keyboard{display:flex;flex-wrap:wrap;gap:4px}.key{min-width:34px;height:68px;border:1px solid #aebbb0;background:#e8eee9;color:#1b231d;cursor:pointer}
.key.black{height:52px;background:#3a4740;color:#fff}.key.on{filter:brightness(1.3)}
.error{min-height:20px;color:#ff9b94}/*sound-lab-style*/
</style>
</head>
<body><main>
<h1>Dandrum Instrument</h1>
<section class="panel"><h2>Public controls</h2><div class="controls" id="controls"></div></section>
<!--sound-lab-panel-->
<section class="panel"><h2>Play notes</h2><div class="keyboard" id="keys"></div></section>
<div class="error" id="error" role="status" aria-live="polite"></div>
</main>
<script src="/shared-instrument-ui.js"></script>
<!--sound-lab-script-->
</body></html>)HTML";
}
