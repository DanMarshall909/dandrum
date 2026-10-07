import assert from 'node:assert/strict';
import test from 'node:test';
import { observePanelHeight } from './panel-height.mjs';

test('frame tracks the unscaled panel height only when its geometry changes', () => {
  let height = 572.5, notifications, observed, disconnected = false, writes = 0;
  const panel = { getBoundingClientRect: () => ({ height }) };
  let value = '';
  const frame = { style: { get height() { return value; }, set height(next) { writes++; value = next; } } };
  class Observer {
    constructor(callback) { notifications = callback; }
    observe(element) { observed = element; }
    disconnect() { disconnected = true; }
  }
  const close = observePanelHeight(panel, frame, Observer);
  assert.equal(observed, panel);
  assert.equal(frame.style.height, '572.5px'); assert.equal(writes, 1);
  notifications(); assert.equal(writes, 1, 'unchanged controls should not rewrite layout');
  height = 516; notifications(); assert.equal(frame.style.height, '516px'); assert.equal(writes, 2);
  close(); assert.equal(disconnected, true);
  height = 600; notifications(); assert.equal(frame.style.height, '516px', 'retired view changed layout');
});
