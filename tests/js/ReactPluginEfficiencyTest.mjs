import assert from 'node:assert/strict';
import test from 'node:test';
import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';

// The spec map accepts tests under tests/. Run the actual shared-helper and
// React suites in isolated Node workers, keeping each jsdom document separate.
const suites = [
  'web/shared/parameter-controller.test.mjs',
  'web/shared/knob-reconciliation.test.mjs',
  'web/shared/meter-view.test.mjs',
  'web/shared/meter-transport.test.mjs',
  'web/shared/prepared-polling.test.mjs',
  'web/shared/panel-height.test.mjs',
  'web/sampler/src/host-parameters.test.mjs',
  'web/sampler/src/host-knob-admission.test.mjs',
  'web/sampler/src/host-knob-face.test.mjs',
];
test('React efficiency acceptance suites pass with their own browser documents', context => {
  const { NODE_TEST_CONTEXT, ...env } = process.env;
  const result = spawnSync(process.execPath, ['--test', ...suites], {
    cwd: fileURLToPath(new URL('../../', import.meta.url)), env, encoding: 'utf8', timeout: 20000,
  });
  assert.equal(result.error, undefined);
  assert.equal(result.status, 0, result.stdout + result.stderr);
  assert.match(result.stdout, /# tests [1-9]\d*/);
  context.diagnostic(result.stdout.split('\n').filter(line => /^# (tests|pass|fail) /.test(line)).join('; '));
});
