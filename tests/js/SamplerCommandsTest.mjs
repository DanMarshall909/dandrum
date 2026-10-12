import assert from 'node:assert/strict';
import { test } from 'node:test';
import { spawnSync } from 'node:child_process';
import { mkdtempSync, readFileSync, writeFileSync, rmSync, existsSync } from 'node:fs';
import { tmpdir } from 'node:os';
import path from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('../../', import.meta.url));
const generator = path.join(root, 'scripts/generate-sampler-commands.mjs');
// Captured from Model::commandIds before extraction; deliberately independent of the registry.
const baseline = `undo redo gesture.cancel page.select state.select guide.select selection.set patch.new patch.load patch.select patch.favourite patch.search
patch.category patch.import patch.export patch.copy patch.rename patch.reload patch.reload-fail patch.discard tree.audition tree.add tree.assign tree.rename
tree.delete tree.duplicate tree.move tree.mute tree.solo tree.expand tree.freeze tree.unfreeze asset.group-toggle asset.remove asset.favourite asset.import
asset.import-many asset.relink asset.replace asset.search-folder asset.select asset.preview asset.search asset.category asset.mock-state automation.mock sample.set sample.snap-zero
sample.mode sample.reverse sample.audition sample.stop sample.freeze sample.unfreeze sample.history-select sample.history-add sample.history-collapse sample.history-bypass sample.history-remove sample.history-move
analysis.start analysis.fail analysis.retry analysis.cancel analysis.apply analysis.discard analysis.sensitivity analysis.count analysis.preview slice.preview slice.add slice.select
slice.split slice.merge slice.delete slice.clear slice.even slice.grid slice.map slice.export-midi slice.set zone.add zone.assign zone.context-select
zone.select zone.set zone.set-free zone.move zone.move-free zone.split zone.delete zone.delete-gap zone.copy zone.copy-range zone.copy-group zone.paste
zone.paste-layer zone.map-root zone.map-velocity zone.map-sequential zone.root selector.axis selector.source selector.lock selector.no-repeat articulation.select articulation.release articulation.pedal
zone.insert zone.add-gap selector.mode selector.reset selector.crossfade layer.output layer.set layer.move layer.delete layer.add layer.mute layer.solo
pad.select pad.assign pad.set pad.output parameter.set parameter.preview parameter.navigate parameter.learn parameter.learn-stop parameter.reset parameter.host filter.mode
voice.policy voice.limit voice.steal voice.same-note voice.choke module.add module.move module.delete module.bypass module.set module.mode amp.detach
amp.replace amp.restore modulator.add modulator.select modulator.rename modulator.delete modulator.set modulator.shape route.add route.delete route.set route.scope
route.combine route.rate route.polarity route.curve route.invert route.bypass route.select route.group route.expand macro.binding-set macro.binding-toggle macro.select
macro.set macro.rename macro.bind macro.unbind macro.invert macro.clear macro.reset macro.learn macro.learn-stop macro.host midi.receive routing.output
routing.inherit chain.gain chain.output chain.mute send.set fx.add fx.move fx.delete fx.bypass fx.set overview.reset inherit.override
inherit.reset scope.select pc.enabled pc.octave telemetry.fail diagnostics.select event.clear freeze.configure autosampler.configure freeze.start freeze.cancel autosampler.start
autosampler.cancel`.trim().split(/\s+/);
function fixture(run) {
  const dir = mkdtempSync(path.join(tmpdir(), 'dandrum-sampler-commands-'));
  const source = path.join(dir, 'commands.json'), slint = path.join(dir, 'Commands.slint'), cpp = path.join(dir, 'CommandIds.h');
  const data = { schemaVersion: 1, commands: ['redo', 'undo', 'asset.import-many'] };
  const write = () => writeFileSync(source, JSON.stringify(data));
  const generate = (...extra) => spawnSync(process.execPath, [generator, '--source', source, '--slint', slint, '--cpp', cpp, ...extra], { encoding: 'utf8' });
  try { write(); run({ dir, source, slint, cpp, data, write, generate }); }
  finally { rmSync(dir, { recursive: true, force: true }); }
}

test('the registry preserves every independently captured Model command in its original order', () => {
  const source = JSON.parse(readFileSync(path.join(root, 'ui/advanced-sampler/domain/commands.json')));
  assert.deepEqual(source, { schemaVersion: 1, commands: baseline });
  assert.equal(baseline.length, 205);
  const result = spawnSync(process.execPath, [generator, '--check'], { encoding: 'utf8' });
  assert.equal(result.status, 0, result.stderr);
  const slint = readFileSync(path.join(root, 'ui/advanced-sampler/domain/Commands.slint'), 'utf8');
  const cpp = readFileSync(path.join(root, 'ui/advanced-sampler/domain/CommandIds.h'), 'utf8');
  assert.deepEqual([...slint.matchAll(/out property <string> ([\w-]+): "([^"]+)";/g)].map(m => [m[1], m[2]]), baseline.map(id => [id.replaceAll('.', '-'), id]));
  assert.deepEqual([...cpp.matchAll(/inline constexpr std::string_view (\w+) = "([^"]+)";/g)].map(m => [m[1], m[2]]), baseline.map(id => [id.replaceAll(/[.-]/g, '_'), id]));
});

test('generated C++ constants and ordered strings compile and round-trip all 205 command values', () => {
  fixture(({ dir }) => {
    const expected = baseline.map(JSON.stringify).join(',');
    const constants = baseline.map(id => `static_assert(commands::${id.replaceAll(/[.-]/g, '_')} == ${JSON.stringify(id)});`).join('\n');
    writeFileSync(path.join(dir, 'consumer.cpp'), `#include "ui/advanced-sampler/domain/CommandIds.h"
      using namespace dandrum::sampler;
      constexpr std::array<std::string_view, 205> expected{${expected}};
      static_assert(commands::ordered_ids == expected);
      ${constants}
      int main() { const auto& ids = commands::ids();
        if (&ids != &commands::ids() || ids.size() != expected.size()) return 1;
        for (std::size_t i = 0; i < ids.size(); ++i) if (ids[i] != expected[i]) return 2;
      }
    `);
    const executable = path.join(dir, 'consumer');
    const build = spawnSync('c++', ['-std=c++20', '-I', root, path.join(dir, 'consumer.cpp'), '-o', executable], { encoding: 'utf8' });
    assert.equal(build.status, 0, build.stderr);
    assert.equal(spawnSync(executable).status, 0);
  });
});

test('custom registry order and updated values reach both generated languages deterministically', () => {
  fixture(({ slint, cpp, data, write, generate }) => {
    assert.equal(generate().status, 0);
    assert.match(readFileSync(slint, 'utf8'), /export global SamplerCommands/);
    assert.deepEqual([...readFileSync(slint, 'utf8').matchAll(/: "([^"]+)";/g)].map(m => m[1]), ['redo', 'undo', 'asset.import-many']);
    assert.match(readFileSync(cpp, 'utf8'), /redo, undo, asset_import_many/);
    const original = readFileSync(cpp, 'utf8'); assert.equal(generate().status, 0); assert.equal(readFileSync(cpp, 'utf8'), original);
    data.commands = ['sample.set', 'zone.move-free']; write(); assert.equal(generate().status, 0);
    assert.match(readFileSync(slint, 'utf8'), /zone-move-free: "zone.move-free"/);
    assert.match(readFileSync(cpp, 'utf8'), /sample_set, zone_move_free/);
    assert.equal(generate('--check').status, 0);
  });
});

test('check mode reports each missing or stale output without rewriting it', () => {
  fixture(({ slint, cpp, generate }) => {
    let checked = generate('--check'); assert.equal(checked.status, 1);
    assert.match(checked.stderr, /Commands.slint.*out of date/); assert.match(checked.stderr, /CommandIds.h.*out of date/);
    assert.equal(existsSync(slint), false); assert.equal(existsSync(cpp), false);
    assert.equal(generate().status, 0); writeFileSync(slint, '// changed Slint\n'); writeFileSync(cpp, '// changed C++\n');
    checked = generate('--check'); assert.equal(checked.status, 1);
    assert.match(checked.stderr, /Commands.slint.*out of date/); assert.match(checked.stderr, /CommandIds.h.*out of date/);
    assert.equal(readFileSync(slint, 'utf8'), '// changed Slint\n'); assert.equal(readFileSync(cpp, 'utf8'), '// changed C++\n');
  });
});

test('invalid registries are rejected before existing outputs can change', () => {
  fixture(({ source, slint, cpp, data, write, generate }) => {
    assert.equal(generate().status, 0); const beforeSlint = readFileSync(slint, 'utf8'), beforeCpp = readFileSync(cpp, 'utf8');
    for (const invalid of [null, { schemaVersion: 2, commands: ['undo'] }, { schemaVersion: 1 },
      { schemaVersion: 1, commands: {} }, { schemaVersion: 1, commands: [] },
      ...[['undo', 'undo'], ['zone.set', 'zone-set'], ['bad id'], ['UPPER'], ['a..b'], [42], ['ids'], ['class']].map(commands => ({ schemaVersion: 1, commands }))]) {
      writeFileSync(source, JSON.stringify(invalid)); const result = generate(); assert.equal(result.status, 1, JSON.stringify(invalid));
      assert.equal(readFileSync(slint, 'utf8'), beforeSlint); assert.equal(readFileSync(cpp, 'utf8'), beforeCpp);
    }
    writeFileSync(source, '{'); assert.equal(generate().status, 1); write(); assert.equal(generate('--unknown').status, 1);
    assert.equal(generate('--source').status, 1);
  });
});

test('output path collisions never overwrite the source or another language output', () => {
  fixture(({ source, slint, cpp, generate }) => {
    const before = readFileSync(source, 'utf8');
    for (const args of [['--slint', source], ['--cpp', source], ['--cpp', slint]]) {
      const result = generate(...args); assert.equal(result.status, 1); assert.match(result.stderr, /paths must be distinct/);
      assert.equal(readFileSync(source, 'utf8'), before); assert.equal(existsSync(slint), false); assert.equal(existsSync(cpp), false);
    }
  });
});
