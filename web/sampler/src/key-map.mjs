import { keyMapLayout } from './model.mjs';

// Adapted from the supplied KeyMap.jsx. Prepared pads are owned snapshot data;
// selection and audition intent belong to the editor. Structural commands and
// observed playback feedback are connected separately when their capabilities exist.
export function createKeyMap(React) {
  const { useLayoutEffect, useRef, useState } = React;
  const h = React.createElement;
  return function PreparedKeyMap({ pads, selectedId, onSelect, onNoteOn, onNoteOff }) {
    const root = useRef(null), scroll = useRef(null), anchor = useRef(null);
    const held = useRef(null), callbacks = useRef(null), zoomControl = useRef(null);
    callbacks.current = { onSelect, onNoteOn, onNoteOff };
    const [width, setWidth] = useState(800);
    const [compact, setCompact] = useState(false);
    const [zoom, setZoom] = useState(1);
    const [pressed, setPressed] = useState(null);
    const layout = keyMapLayout(pads, width, zoom, compact);
    const selected = pads.find(pad => pad.id === selectedId);

    const release = () => {
      if (!held.current) return;
      const { note } = held.current;
      held.current = null;
      setPressed(null);
      callbacks.current.onNoteOff(note);
    };
    const releaseOwner = owner => { if (held.current?.owner === owner) release(); };
    const press = (note, velocity, owner) => {
      release();
      held.current = { note, velocity, owner };
      setPressed({ note, velocity });
      callbacks.current.onNoteOn(note, velocity / 127);
    };
    const changeZoom = (requested, pointerX) => {
      const next = Math.max(1, Math.min(8, Math.round(requested * 100) / 100));
      if (next === zoom || !layout) return;
      const viewport = scroll.current;
      const x = pointerX ?? viewport.clientWidth / 2;
      anchor.current = { x, note: (viewport.scrollLeft + x - 30) / layout.keyWidth };
      setZoom(next);
    };
    zoomControl.current = { zoom, changeZoom };
    useLayoutEffect(() => {
      const measure = () => {
        setWidth(Math.max(31, root.current.clientWidth));
        setCompact(window.innerWidth <= 900);
      };
      const wheel = event => {
        if (!event.ctrlKey && !event.metaKey) return;
        event.preventDefault();
        const current = zoomControl.current;
        current.changeZoom(current.zoom * Math.exp(-event.deltaY * 0.004),
          event.clientX - scroll.current.getBoundingClientRect().left);
      };
      const visibility = release;
      const escape = event => { if (event.key === 'Escape') release(); };
      const observer = new ResizeObserver(measure);
      observer.observe(root.current); measure();
      scroll.current.addEventListener('wheel', wheel, { passive: false });
      window.addEventListener('resize', measure);
      window.addEventListener('blur', release);
      window.addEventListener('pagehide', release);
      window.addEventListener('keydown', escape);
      document.addEventListener('visibilitychange', visibility);
      const viewport = scroll.current;
      return () => {
        observer.disconnect();
        viewport.removeEventListener('wheel', wheel);
        window.removeEventListener('resize', measure);
        window.removeEventListener('blur', release);
        window.removeEventListener('pagehide', release);
        window.removeEventListener('keydown', escape);
        document.removeEventListener('visibilitychange', visibility);
        release();
      };
    }, []);
    useLayoutEffect(() => {
      if (!anchor.current || !layout) return;
      scroll.current.scrollLeft = anchor.current.note * layout.keyWidth + 30 - anchor.current.x;
      anchor.current = null;
    }, [layout?.keyWidth]);

    const keyUp = (event, owner) => {
      if (event.key !== 'Enter' && event.key !== ' ') return;
      event.preventDefault(); releaseOwner(owner);
    };
    const zoneKey = (event, pad, index) => {
      const direction = { ArrowLeft: -1, ArrowDown: -1, ArrowRight: 1, ArrowUp: 1 }[event.key];
      if (direction) {
        event.preventDefault();
        const next = Math.max(0, Math.min(pads.length - 1, index + direction));
        root.current.querySelectorAll('.dd-key-zone')[next].focus();
        return;
      }
      if (event.key !== 'Enter' && event.key !== ' ') return;
      event.preventDefault();
      if (event.repeat) return;
      callbacks.current.onSelect(pad);
      press(pad.keyLow, Math.round(pad.velocity * 127), `zone:${pad.id}`);
    };
    const pointerKey = (event, note) => {
      if (event.button !== 0) return;
      event.preventDefault();
      event.currentTarget.focus();
      event.currentTarget.setPointerCapture(event.pointerId);
      const bounds = event.currentTarget.getBoundingClientRect();
      const velocity = Math.max(1, Math.min(127,
        Math.round(24 + (event.clientY - bounds.top) / bounds.height * 103)));
      press(note, velocity, `pointer:${event.pointerId}:${note}`);
    };
    const keyboardKey = (event, note) => {
      if (event.key !== 'Enter' && event.key !== ' ') return;
      event.preventDefault();
      if (event.repeat) return;
      const velocity = selected && note >= selected.keyLow && note <= selected.keyHigh
        ? Math.round(selected.velocity * 127) : 96;
      press(note, velocity, `key:${note}`);
    };
    const pianoKey = (key, black) => h('button', {
      key: key.note, type: 'button', className: `dd-piano-key ${black ? 'black' : 'white'}`,
      'data-note': key.note, 'aria-label': `Key ${key.note} ${key.name}`,
      'aria-pressed': pressed?.note === key.note,
      disabled: key.note < layout.lowNote || key.note > layout.highNote,
      style: { left: key.left, width: key.width },
      onPointerDown: event => pointerKey(event, key.note),
      onPointerUp: event => releaseOwner(`pointer:${event.pointerId}:${key.note}`),
      onPointerCancel: event => releaseOwner(`pointer:${event.pointerId}:${key.note}`),
      onLostPointerCapture: event => releaseOwner(`pointer:${event.pointerId}:${key.note}`),
      onBlur: release,
      onKeyDown: event => keyboardKey(event, key.note),
      onKeyUp: event => keyUp(event, `key:${key.note}`),
    }, !black && key.note % 12 === 0 ? key.name : null);

    return h('div', { ref: root, className: 'dd-key-map',
      'data-low-note': layout?.lowNote, 'data-high-note': layout?.highNote,
      'data-compact': compact, 'data-zoom': zoom },
      h('div', { className: 'dd-key-map-heading' },
        h('span', { className: 'dd-key-map-readout' }, selected
          ? `KEY ${selected.keyLow}–${selected.keyHigh} · VEL ${selected.velocityLow}–${selected.velocityHigh}`
          : 'No zone selected'),
        h('span', { className: 'dd-key-map-intent', 'aria-live': 'polite' }, pressed
          ? `EDITOR PRESS ${pressed.note} · VEL ${pressed.velocity}` : ''),
        h('div', { className: 'dd-key-map-zoom', role: 'group', 'aria-label': 'Key map zoom' },
          h('button', { type: 'button', 'aria-label': 'Zoom out', disabled: zoom <= 1 || !layout,
            onClick: () => changeZoom(zoom / 1.5) }, '−'),
          h('span', null, `${Math.round(zoom * 100)}%`),
          h('button', { type: 'button', 'aria-label': 'Zoom in', disabled: zoom >= 8 || !layout,
            onClick: () => changeZoom(zoom * 1.5) }, '+'),
          h('button', { type: 'button', 'aria-label': 'Zoom to fit', disabled: !layout,
            onClick: () => changeZoom(1) }, 'Fit'))),
      h('div', { ref: scroll, className: 'dd-key-map-scroll' }, layout
        ? h('div', { className: 'dd-key-map-content',
          style: { gridTemplateColumns: `30px ${layout.width}px` } },
          h('div', { className: 'dd-key-map-axis', style: { height: layout.gridHeight } },
            [127, 96, 64, 32, 1].map(velocity => h('span', { key: velocity,
              style: { top: Math.max(0, Math.min(layout.gridHeight - 12,
                (127 - velocity) / 127 * layout.gridHeight - 6)) } }, velocity))),
          h('div', { className: 'dd-key-map-grid', style: { width: layout.width, height: layout.gridHeight } },
            layout.notes.map(key => h('div', { key: key.note,
              className: `dd-key-column ${key.black ? 'black' : ''}`,
              style: { left: key.left, width: key.width } })),
            [96, 64, 32].map(velocity => h('div', { key: velocity, className: 'dd-key-velocity-line',
              style: { top: (127 - velocity) / 127 * layout.gridHeight } })),
            layout.zones.map((zone, index) => {
              const pad = pads[index];
              return h('button', { key: pad.id, type: 'button', className: 'dd-key-zone',
                'data-pad-id': pad.id, 'data-velocity-low': pad.velocityLow, 'data-velocity-high': pad.velocityHigh,
                'aria-label': `${pad.label}, MIDI ${pad.keyLow} to ${pad.keyHigh}, velocity ${pad.velocityLow} to ${pad.velocityHigh}`,
                'aria-pressed': pad.id === selectedId,
                style: { left: zone.left, top: zone.top, width: zone.width, height: zone.height },
                onFocus: () => callbacks.current.onSelect(pad),
                onClick: () => callbacks.current.onSelect(pad),
                onKeyDown: event => zoneKey(event, pad, index),
                onKeyUp: event => keyUp(event, `zone:${pad.id}`), onBlur: release,
              }, zone.width >= 34 && zone.height >= 16
                ? h('span', { className: 'dd-key-zone-label' }, pad.label) : null);
            })),
          h('div', { className: 'dd-key-map-axis' }),
          h('div', { className: 'dd-key-map-rail', style: { width: layout.width } },
            layout.notes.filter(key => pads.some(pad => key.note >= pad.keyLow && key.note <= pad.keyHigh))
              .map(key => h('i', { key: key.note,
                className: selected && key.note >= selected.keyLow && key.note <= selected.keyHigh ? 'selected' : '',
                style: { left: key.left, width: key.width } }))),
          h('div', { className: 'dd-key-map-axis' }),
          h('div', { className: 'dd-key-map-piano', role: 'group', 'aria-label': 'Editor audition keyboard',
            style: { width: layout.width, height: layout.keyboardHeight } },
            layout.whiteKeys.map(key => pianoKey(key, false)),
            layout.notes.filter(key => key.black).map(key => pianoKey(key, true))))
        : h('p', { className: 'dd-key-map-empty', role: 'status' }, 'No prepared zones')),
      h('p', { className: 'dd-key-map-capability' }, 'Playback feedback unavailable'));
  };
}
