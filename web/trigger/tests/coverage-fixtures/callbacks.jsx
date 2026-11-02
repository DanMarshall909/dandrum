import React from 'react';
import {createRoot} from 'react-dom/client';
// 😀 keeps the fixture sensitive to UTF-16 versus byte offsets.
export function missed() {
  return 'never';
}
function choose(first) {
  if (first) {
    return 'first';
  }
  return 'other';
}
function called({currentTarget:target}) {
  target.textContent = 'hit 😀';
}
createRoot(document.getElementById('root')).render(<button onClick={called}>{choose(true)}</button>);
