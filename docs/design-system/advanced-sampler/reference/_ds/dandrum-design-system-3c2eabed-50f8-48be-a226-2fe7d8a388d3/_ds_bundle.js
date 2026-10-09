/* @ds-bundle: {"format":4,"namespace":"DandrumDesignSystem_3c2eab","components":[{"name":"Button","sourcePath":"components/controls/Button.jsx"},{"name":"IconButton","sourcePath":"components/controls/Button.jsx"},{"name":"Knob","sourcePath":"components/controls/Knob.jsx"},{"name":"NumericField","sourcePath":"components/controls/NumericField.jsx"},{"name":"Slider","sourcePath":"components/controls/Slider.jsx"},{"name":"Toggle","sourcePath":"components/controls/Toggle.jsx"},{"name":"Switch","sourcePath":"components/controls/Toggle.jsx"},{"name":"KeyMap","sourcePath":"components/display/KeyMap.jsx"},{"name":"LayerStack","sourcePath":"components/display/LayerStack.jsx"},{"name":"ListRow","sourcePath":"components/display/ListRow.jsx"},{"name":"PropertyRow","sourcePath":"components/display/ListRow.jsx"},{"name":"ParamLabel","sourcePath":"components/display/ListRow.jsx"},{"name":"Meter","sourcePath":"components/display/Meter.jsx"},{"name":"MOD_SLOTS","sourcePath":"components/display/ModIndicator.jsx"},{"name":"ModGlyph","sourcePath":"components/display/ModIndicator.jsx"},{"name":"ModIndicator","sourcePath":"components/display/ModIndicator.jsx"},{"name":"OutputBusses","sourcePath":"components/display/OutputBusses.jsx"},{"name":"PadCell","sourcePath":"components/display/PadCell.jsx"},{"name":"WaveformPanel","sourcePath":"components/display/WaveformPanel.jsx"},{"name":"StatusMessage","sourcePath":"components/feedback/StatusMessage.jsx"},{"name":"Tooltip","sourcePath":"components/feedback/StatusMessage.jsx"},{"name":"EmptyState","sourcePath":"components/feedback/StatusMessage.jsx"},{"name":"ICON_NAMES","sourcePath":"components/icons/Icon.jsx"},{"name":"Icon","sourcePath":"components/icons/Icon.jsx"},{"name":"Panel","sourcePath":"components/layout/Panel.jsx"},{"name":"Rollout","sourcePath":"components/layout/Panel.jsx"},{"name":"SectionHeading","sourcePath":"components/layout/Panel.jsx"},{"name":"ContextMenu","sourcePath":"components/navigation/ContextMenu.jsx"},{"name":"MenuButton","sourcePath":"components/navigation/ContextMenu.jsx"},{"name":"Tabs","sourcePath":"components/navigation/Tabs.jsx"},{"name":"SegmentedControl","sourcePath":"components/navigation/Tabs.jsx"}],"sourceHashes":{"components/controls/Button.jsx":"44e6b1747c46","components/controls/Knob.jsx":"cc697f2a99e7","components/controls/NumericField.jsx":"3ffcbd8ba68f","components/controls/Slider.jsx":"8ab90650afc3","components/controls/Toggle.jsx":"9fc7528cfa4e","components/display/KeyMap.jsx":"612300004520","components/display/LayerStack.jsx":"a86c12fb75d5","components/display/ListRow.jsx":"4aa50bf3baaa","components/display/Meter.jsx":"c1a91a3a8df3","components/display/ModIndicator.jsx":"fe25ff78bf4b","components/display/OutputBusses.jsx":"9f71f7b1959a","components/display/PadCell.jsx":"a3a38c86b407","components/display/WaveformPanel.jsx":"d2000a4511b9","components/feedback/StatusMessage.jsx":"33f8d2ee71e0","components/icons/Icon.jsx":"dfbed32ba5bd","components/layout/Panel.jsx":"41edf043432a","components/navigation/ContextMenu.jsx":"6b14855384be","components/navigation/Tabs.jsx":"2ddd7e6976e1","tools/dev-bundle-fallback.js":"dd7aebb79959","tools/tweaks-panel.jsx":"d259e3a86f73","ui_kits/das-sampler/App.jsx":"fd18586f93ea","ui_kits/das-sampler/Header.jsx":"12077178e271","ui_kits/das-sampler/LiveControls.jsx":"6cea3ddaac2d","ui_kits/das-sampler/PadDetails.jsx":"187993f680a2","ui_kits/das-sampler/PadGrid.jsx":"510664d916dc","ui_kits/das-sampler/WaveView.jsx":"5352eedbc3ba","ui_kits/das-sampler/data.js":"b2e7912b4b12"},"inlinedExternals":[],"unexposedExports":[{"name":"makePeaks","sourcePath":"components/display/WaveformPanel.jsx"},{"name":"noteName","sourcePath":"components/display/PadCell.jsx"}]} */

(() => {

const __ds_ns = (window.DandrumDesignSystem_3c2eab = window.DandrumDesignSystem_3c2eab || {});

const __ds_scope = {};

(__ds_ns.__errors = __ds_ns.__errors || []);

// components/controls/NumericField.jsx
try { (() => {
const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/**
 * Numeric field: drag vertically to scrub, click to type, arrows/wheel to step, double-click resets.
 * Mono value, unit in tertiary. `readOnly` renders prepared (non-live) values with a dotted outline.
 */
function NumericField({
  value = 0,
  min = 0,
  max = 100,
  step = 1,
  defaultValue,
  unit,
  label,
  format,
  width,
  compact = false,
  readOnly = false,
  disabled = false,
  focused = false,
  hostAutomated = false,
  onChange,
  onContextMenu
}) {
  const [editing, setEditing] = React.useState(false);
  const [text, setText] = React.useState('');
  const [focus, setFocus] = React.useState(false);
  const [hover, setHover] = React.useState(false);
  const drag = React.useRef(null);
  const fmt = format || (v => Number.isInteger(step) ? String(Math.round(v)) : v.toFixed(2));
  const clamp = v => Math.max(min, Math.min(max, v));
  const set = v => {
    if (!readOnly && !disabled && onChange) onChange(clamp(Math.round(v / step) * step));
  };
  const live = !readOnly && !disabled;
  const h = compact ? 22 : 24;
  const commit = () => {
    const n = parseFloat(text);
    if (!Number.isNaN(n)) set(n);
    setEditing(false);
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'inline-flex',
      flexDirection: 'column',
      gap: 3
    }
  }, label && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: disabled ? 'var(--text-disabled)' : 'var(--text-secondary)'
    }
  }, label), /*#__PURE__*/React.createElement("div", {
    role: "spinbutton",
    tabIndex: disabled ? -1 : 0,
    "aria-label": label,
    "aria-valuenow": value,
    "aria-valuemin": min,
    "aria-valuemax": max,
    "aria-readonly": readOnly || undefined,
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    onPointerDown: e => {
      if (!live || editing || e.button !== 0) return;
      e.currentTarget.setPointerCapture(e.pointerId);
      drag.current = {
        y: e.clientY,
        v: value,
        moved: false
      };
    },
    onPointerMove: e => {
      if (!drag.current) return;
      const dy = drag.current.y - e.clientY;
      if (Math.abs(dy) > 2) drag.current.moved = true;
      set(drag.current.v + Math.round(dy / (e.shiftKey ? 12 : 4)) * step);
    },
    onPointerUp: () => {
      const d = drag.current;
      drag.current = null;
      if (d && !d.moved && live) {
        setText(fmt(value));
        setEditing(true);
      }
    },
    onDoubleClick: () => defaultValue != null && set(defaultValue),
    onKeyDown: e => {
      if (!live || editing) return;
      if (e.key === 'ArrowUp') {
        set(value + step * (e.shiftKey ? 10 : 1));
        e.preventDefault();
      } else if (e.key === 'ArrowDown') {
        set(value - step * (e.shiftKey ? 10 : 1));
        e.preventDefault();
      } else if (e.key === 'Enter') {
        setText(fmt(value));
        setEditing(true);
      } else if (e.key === 'F10' && e.shiftKey || e.key === 'ContextMenu') {
        e.preventDefault();
        onContextMenu && onContextMenu({
          clientX: e.currentTarget.getBoundingClientRect().left,
          clientY: e.currentTarget.getBoundingClientRect().bottom,
          preventDefault() {},
          fromKeyboard: true
        });
      }
    },
    onContextMenu: e => {
      e.preventDefault();
      if (live && onContextMenu) onContextMenu(e);
    },
    style: {
      height: h,
      width: width ?? (compact ? 60 : 72),
      boxSizing: 'border-box',
      padding: '0 6px',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: 4,
      background: readOnly ? 'transparent' : 'var(--surface-well)',
      borderRadius: 'var(--radius-1)',
      outline: 'none',
      border: readOnly ? '1px dashed var(--border-control)' : `1px solid ${editing ? 'var(--dd-paper-2)' : hover && live ? 'var(--border-strong)' : 'var(--border-control)'}`,
      boxShadow: focus || focused ? focusRing : readOnly ? 'none' : 'var(--inset-well)',
      cursor: live ? editing ? 'text' : 'ns-resize' : 'default',
      touchAction: 'none'
    }
  }, editing ? /*#__PURE__*/React.createElement("input", {
    autoFocus: true,
    value: text,
    onChange: e => setText(e.target.value),
    onBlur: commit,
    onKeyDown: e => {
      if (e.key === 'Enter') commit();
      if (e.key === 'Escape') setEditing(false);
    },
    style: {
      width: '100%',
      background: 'transparent',
      border: 0,
      outline: 'none',
      color: 'var(--text-primary)',
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-value)',
      padding: 0
    }
  }) : /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)',
      color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : readOnly ? 'var(--text-secondary)' : 'var(--text-primary)',
      whiteSpace: 'nowrap'
    }
  }, fmt(value)), unit && !editing && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      color: 'var(--text-tertiary)'
    }
  }, unit)));
}
Object.assign(__ds_scope, { NumericField });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/NumericField.jsx", error: String((e && e.message) || e) }); }

// components/controls/Toggle.jsx
try { (() => {
const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/** On/off toggle (pill track + square-ish thumb). Use for binary live options. Label sits right. */
function Toggle({
  checked = false,
  label,
  disabled = false,
  focused = false,
  hostAutomated = false,
  onChange,
  compact = false
}) {
  const [focus, setFocus] = React.useState(false);
  const w = compact ? 28 : 32,
    h = compact ? 16 : 18,
    t = h - 6;
  const on = checked && !disabled;
  return /*#__PURE__*/React.createElement("label", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 8,
      minHeight: 24,
      cursor: disabled ? 'default' : 'pointer',
      userSelect: 'none'
    }
  }, /*#__PURE__*/React.createElement("button", {
    type: "button",
    role: "switch",
    "aria-checked": checked,
    disabled: disabled,
    onClick: () => onChange && onChange(!checked),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      width: w,
      height: h,
      padding: 0,
      position: 'relative',
      borderRadius: 'var(--radius-round)',
      outline: 'none',
      boxSizing: 'border-box',
      background: on ? hostAutomated ? 'var(--dd-host)' : 'var(--dd-paper-1)' : 'var(--dd-ink-0)',
      border: `1px solid ${on ? 'transparent' : 'var(--border-control)'}`,
      cursor: 'inherit',
      boxShadow: focus || focused ? focusRing : 'none',
      opacity: disabled ? 0.5 : 1
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: 2,
      left: checked ? w - t - 4 : 2,
      width: t,
      height: t,
      borderRadius: 'var(--radius-round)',
      background: on ? 'var(--dd-ink-1)' : disabled ? 'var(--dd-ink-5)' : 'var(--dd-paper-3)',
      transition: 'left 90ms ease-out'
    }
  })), label && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-label)' : 'var(--type-body)',
      fontWeight: 500,
      color: disabled ? 'var(--text-disabled)' : 'var(--text-primary)'
    }
  }, label));
}

/**
 * Hardware-style latching switch: a labelled key with an LED bar. Use for mode switches that
 * read like instrument buttons (Reverse, Loop, Mono). Pairs LED + text so state never relies on colour.
 */
function Switch({
  on = false,
  label,
  disabled = false,
  focused = false,
  onChange,
  compact = false,
  width
}) {
  const [hover, setHover] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-pressed": on,
    disabled: disabled,
    onClick: () => onChange && onChange(!on),
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      height: compact ? 24 : 28,
      minWidth: width ?? (compact ? 52 : 64),
      padding: '0 8px',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      background: disabled ? 'var(--dd-ink-3)' : hover ? 'var(--dd-ink-5)' : 'var(--dd-ink-4)',
      boxSizing: 'border-box',
      border: '1px solid var(--border-control)',
      borderRadius: 'var(--radius-2)',
      outline: 'none',
      cursor: disabled ? 'default' : 'pointer',
      boxShadow: focus || focused ? focusRing : 'var(--highlight-cap)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      width: 3,
      height: compact ? 10 : 12,
      borderRadius: 1,
      background: disabled ? 'var(--dd-ink-6)' : on ? 'var(--dd-vermilion)' : 'var(--dd-ink-0)',
      boxShadow: on && !disabled ? '0 0 0 1px var(--dd-vermilion-lo)' : 'inset 0 0 0 1px var(--dd-line-2)'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-micro)' : 'var(--type-label)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: disabled ? 'var(--text-disabled)' : on ? 'var(--text-primary)' : 'var(--text-secondary)'
    }
  }, label));
}
Object.assign(__ds_scope, { Toggle, Switch });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/Toggle.jsx", error: String((e && e.message) || e) }); }

// components/display/KeyMap.jsx
try { (() => {
const SM_NOTE_NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
const noteName = n => SM_NOTE_NAMES[n % 12] + (Math.floor(n / 12) - 1);
const SM_BLACK = [1, 3, 6, 8, 10];
const smIsBlack = n => SM_BLACK.includes((n % 12 + 12) % 12);
const SM_WHITE_IDX = {
  0: 0,
  2: 1,
  4: 2,
  5: 3,
  7: 4,
  9: 5,
  11: 6
};
const smClamp = (v, a, b) => Math.max(a, Math.min(b, v));
const SM_AXIS = 30;
const smLabel = {
  fontFamily: 'var(--font-ui)',
  fontSize: 'var(--type-micro)',
  fontWeight: 600,
  letterSpacing: '0.08em',
  textTransform: 'uppercase',
  color: 'var(--text-tertiary)'
};
const smValue = {
  fontFamily: 'var(--font-value)',
  fontSize: 'var(--type-micro)',
  color: 'var(--text-primary)',
  whiteSpace: 'nowrap'
};

/**
 * Key × velocity map with an aligned, zoomable piano keyboard.
 * Each zone routes a key/velocity range to a source: a synth engine, a sample, or any compatible Dandrum patch.
 * Overlapping zones layer; identical zones draw as an offset stack and the header lists every layer under the selection.
 * Zones are rectangles: x = MIDI key range, y = velocity range (top = 127).
 * Zoom: − / + / Fit in the header, or Ctrl/Cmd + wheel (anchored at the pointer). Scroll horizontally when zoomed.
 * Drag a zone body to move it, drag an edge or corner to resize; arrows nudge (Shift resizes the high edge).
 * Play keys on the keyboard: lower on the key = louder. Matching zones light up.
 */
function KeyMap({
  zones = [],
  selectedId,
  onSelect,
  onChange,
  onNoteOn,
  onNoteOff,
  lowNote = 24,
  highNote = 96,
  keyWidth,
  gridHeight = 140,
  keyboardHeight = 56,
  editable = true,
  title = 'Key map',
  zoom: zoomProp,
  onZoomChange,
  minZoom = 1,
  maxZoom = 8,
  style
}) {
  const [local, setLocal] = React.useState(zones);
  const localRef = React.useRef(zones);
  React.useEffect(() => {
    setLocal(zones);
    localRef.current = zones;
  }, [zones]);
  const [selLocal, setSelLocal] = React.useState(selectedId ?? zones[0]?.id);
  React.useEffect(() => {
    if (selectedId !== undefined) setSelLocal(selectedId);
  }, [selectedId]);
  const [hover, setHover] = React.useState(null);
  const [held, setHeld] = React.useState(null);
  const [lit, setLit] = React.useState([]);
  const [drag, setDrag] = React.useState(null);
  const [focusId, setFocusId] = React.useState(null);
  const wrapRef = React.useRef(null);
  const scrollRef = React.useRef(null);
  const anchorRef = React.useRef(null);
  const [zoomLocal, setZoomLocal] = React.useState(1);
  const zoom = zoomProp ?? zoomLocal;
  const setZoom = (z, anchorX) => {
    const nz = smClamp(Math.round(z * 100) / 100, minZoom, maxZoom);
    if (nz === zoom) return;
    const el = scrollRef.current;
    if (el) {
      const ax = anchorX ?? el.clientWidth / 2;
      anchorRef.current = {
        ax,
        note: (el.scrollLeft + ax - SM_AXIS) / k
      };
    }
    setZoomLocal(nz);
    onZoomChange && onZoomChange(nz);
  };
  const [avail, setAvail] = React.useState(800);
  React.useLayoutEffect(() => {
    if (!wrapRef.current) return;
    const ro = new ResizeObserver(([e]) => setAvail(e.contentRect.width));
    ro.observe(wrapRef.current);
    return () => ro.disconnect();
  }, []);
  const count = highNote - lowNote + 1;
  const k = (keyWidth ?? Math.max(4, (avail - SM_AXIS) / count)) * zoom;
  const W = count * k,
    H = gridHeight;
  const xOf = n => (n - lowNote) * k;
  const yTop = v => (127 - v) / 127 * H;
  const yBot = v => (128 - v) / 127 * H;
  React.useLayoutEffect(() => {
    const a = anchorRef.current,
      el = scrollRef.current;
    if (!a || !el) return;
    el.scrollLeft = a.note * k + SM_AXIS - a.ax;
    anchorRef.current = null;
  }, [k]);
  const zoomRef = React.useRef();
  zoomRef.current = {
    zoom,
    setZoom
  };
  React.useEffect(() => {
    const el = scrollRef.current;
    if (!el) return;
    const wheel = e => {
      if (!(e.ctrlKey || e.metaKey)) return;
      e.preventDefault();
      const r = el.getBoundingClientRect();
      const {
        zoom: z,
        setZoom: sz
      } = zoomRef.current;
      sz(z * Math.exp(-e.deltaY * 0.004), e.clientX - r.left);
    };
    el.addEventListener('wheel', wheel, {
      passive: false
    });
    return () => el.removeEventListener('wheel', wheel);
  }, []);
  const select = id => {
    setSelLocal(id);
    onSelect && onSelect(id);
  };
  const update = (id, patch) => {
    const next = localRef.current.map(z => z.id === id ? {
      ...z,
      ...patch
    } : z);
    localRef.current = next;
    setLocal(next);
    onChange && onChange(next, id);
  };
  const geom = (o, mode, dn, dv) => {
    if (mode === 'move') {
      const span = o.hi - o.lo,
        vspan = o.velHi - o.velLo;
      const lo = smClamp(o.lo + dn, lowNote, highNote - span);
      const velLo = smClamp(o.velLo + dv, 1, 127 - vspan);
      const p = {
        lo,
        hi: lo + span,
        velLo,
        velHi: velLo + vspan
      };
      if (o.root != null) p.root = smClamp(o.root + (lo - o.lo), 0, 127);
      return p;
    }
    const p = {};
    if (mode.includes('l')) p.lo = smClamp(o.lo + dn, lowNote, o.hi);
    if (mode.includes('r')) p.hi = smClamp(o.hi + dn, o.lo, highNote);
    if (mode.includes('t')) p.velHi = smClamp(o.velHi + dv, o.velLo, 127);
    if (mode.includes('b')) p.velLo = smClamp(o.velLo + dv, 1, o.velHi);
    return p;
  };
  React.useEffect(() => {
    if (!drag) return;
    const move = e => {
      const dn = Math.round((e.clientX - drag.x0) / k);
      const dv = Math.round(-(e.clientY - drag.y0) / H * 127);
      update(drag.id, geom(drag.orig, drag.mode, dn, dv));
    };
    const up = () => setDrag(null);
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
    return () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
    };
  }, [drag, k, H]);
  const startDrag = (e, z, mode) => {
    if (e.button !== 0) return;
    e.stopPropagation();
    e.preventDefault();
    select(z.id);
    if (!editable) return;
    setDrag({
      id: z.id,
      mode,
      x0: e.clientX,
      y0: e.clientY,
      orig: {
        ...z
      }
    });
  };
  const onZoneKey = (e, z) => {
    if (!editable) return;
    const d = {
      ArrowLeft: [-1, 0],
      ArrowRight: [1, 0],
      ArrowUp: [0, 1],
      ArrowDown: [0, -1]
    }[e.key];
    if (!d) return;
    e.preventDefault();
    const step = e.altKey ? 8 : 1;
    const mode = e.shiftKey ? d[0] ? 'r' : 't' : 'move';
    update(z.id, geom(z, mode, d[0] * step, d[1] * step));
  };
  const gridPoint = e => {
    const r = e.currentTarget.getBoundingClientRect();
    const x = e.clientX - r.left,
      y = e.clientY - r.top;
    return {
      note: smClamp(Math.floor(x / k) + lowNote, lowNote, highNote),
      vel: smClamp(127 - Math.floor(y / H * 127), 1, 127)
    };
  };
  const noteOn = (e, n) => {
    if (e.button !== 0) return;
    e.preventDefault();
    e.currentTarget.setPointerCapture && e.currentTarget.setPointerCapture(e.pointerId);
    const r = e.currentTarget.getBoundingClientRect();
    const vel = smClamp(Math.round(24 + (e.clientY - r.top) / r.height * 103), 1, 127);
    const hits = localRef.current.filter(z => n >= z.lo && n <= z.hi && vel >= z.velLo && vel <= z.velHi);
    setHeld({
      note: n,
      vel
    });
    setLit(hits.map(z => z.id));
    if (hits.length) select(hits[hits.length - 1].id);
    onNoteOn && onNoteOn(n, vel);
  };
  const noteOff = () => {
    if (!held) return;
    onNoteOff && onNoteOff(held.note);
    setHeld(null);
    setLit([]);
  };
  const selZone = local.find(z => z.id === selLocal);
  const ordered = [...local.filter(z => z.id !== selLocal), ...(selZone ? [selZone] : [])];
  const sameRect = (a, b) => a.lo === b.lo && a.hi === b.hi && a.velLo === b.velLo && a.velHi === b.velHi;
  const stackIdx = z => {
    const i = local.indexOf(z);
    return local.slice(0, i).filter(o => sameRect(o, z)).length;
  };
  const stackCount = z => local.filter(o => sameRect(o, z)).length;
  const layered = selZone ? local.filter(o => o.lo <= selZone.hi && o.hi >= selZone.lo && o.velLo <= selZone.velHi && o.velHi >= selZone.velLo) : [];
  const inSel = n => selZone && n >= selZone.lo && n <= selZone.hi;
  const mapped = n => local.some(z => n >= z.lo && n <= z.hi);
  const notes = Array.from({
    length: count
  }, (_, i) => lowNote + i);
  const whites = [];
  for (let n = lowNote - 1; n <= highNote + 1; n++) if (!smIsBlack(n) && n >= 0 && n <= 127) whites.push(n);
  const whiteLeft = n => (Math.floor(n / 12) * 12 + SM_WHITE_IDX[n % 12] * 12 / 7 - lowNote) * k;
  const whiteW = 12 / 7 * k;
  const readout = hover ? hover : held;
  const scroll = W + SM_AXIS > avail + 0.5;
  const zb = {
    width: 20,
    height: 20,
    padding: 0,
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'center',
    background: 'var(--surface-control)',
    border: '1px solid var(--border-control)',
    borderRadius: 'var(--radius-2, 4px)',
    color: 'var(--text-secondary)',
    fontFamily: 'var(--font-value)',
    fontSize: 'var(--type-label)',
    lineHeight: 1,
    cursor: 'pointer'
  };
  const cursorFor = m => ({
    move: drag ? 'grabbing' : 'grab',
    l: 'ew-resize',
    r: 'ew-resize',
    t: 'ns-resize',
    b: 'ns-resize',
    lt: 'nwse-resize',
    rb: 'nwse-resize',
    rt: 'nesw-resize',
    lb: 'nesw-resize'
  })[m];
  const range = z => z.lo === z.hi ? noteName(z.lo) : `${noteName(z.lo)}–${noteName(z.hi)}`;
  const rangeNum = z => z.lo === z.hi ? `MIDI ${z.lo}` : `MIDI ${z.lo}–${z.hi}`;
  return /*#__PURE__*/React.createElement("div", {
    ref: wrapRef,
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 6,
      minWidth: 0,
      userSelect: 'none',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 12,
      minHeight: 18,
      flexWrap: 'wrap'
    }
  }, title && /*#__PURE__*/React.createElement("span", {
    style: smLabel
  }, title), !editable && /*#__PURE__*/React.createElement("span", {
    title: "Prepared setting. Edit the patch and reload to change.",
    style: {
      ...smLabel,
      padding: '1px 4px',
      border: '1px dashed var(--border-strong)',
      borderRadius: 2
    }
  }, "Prepared"), selZone && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 10,
      alignItems: 'baseline',
      flexWrap: 'wrap'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-label)',
      fontWeight: 600,
      color: 'var(--text-primary)',
      whiteSpace: 'nowrap'
    }
  }, layered.length > 1 ? '' : selZone.name), layered.length <= 1 && selZone.source && /*#__PURE__*/React.createElement("span", {
    style: {
      ...smLabel,
      padding: '1px 4px',
      background: 'var(--dd-ink-0)',
      borderRadius: 2,
      color: 'var(--text-secondary)'
    }
  }, selZone.source), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 4,
      alignItems: 'baseline',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: smLabel
  }, "Key"), /*#__PURE__*/React.createElement("span", {
    style: smValue,
    title: rangeNum(selZone)
  }, range(selZone))), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 4,
      alignItems: 'baseline',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: smLabel
  }, "Vel"), /*#__PURE__*/React.createElement("span", {
    style: smValue
  }, selZone.velLo, "\u2013", selZone.velHi)), selZone.root != null && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 4,
      alignItems: 'baseline',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: smLabel
  }, "Root"), /*#__PURE__*/React.createElement("span", {
    style: smValue,
    title: `MIDI ${selZone.root}`
  }, noteName(selZone.root)))), layered.length > 1 && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      flexWrap: 'wrap'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: smLabel
  }, "Layered"), layered.map(z => /*#__PURE__*/React.createElement("button", {
    key: z.id,
    type: "button",
    onClick: () => select(z.id),
    "aria-pressed": z.id === selLocal,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 5,
      height: 20,
      padding: '0 6px',
      background: z.id === selLocal ? 'var(--dd-vermilion-wash)' : 'var(--surface-control)',
      border: `1px solid ${z.id === selLocal ? 'var(--dd-vermilion)' : 'var(--border-control)'}`,
      borderRadius: 'var(--radius-2, 4px)',
      cursor: 'pointer',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 600,
      color: 'var(--text-primary)',
      whiteSpace: 'nowrap'
    }
  }, z.name))), /*#__PURE__*/React.createElement("span", {
    style: {
      ...smValue,
      marginLeft: 'auto',
      color: 'var(--text-tertiary)',
      minWidth: 0,
      textAlign: 'right'
    },
    title: readout ? `MIDI ${readout.note}` : undefined
  }, readout ? `${noteName(readout.note)} · vel ${readout.vel}` : ''), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 4
    },
    title: "Zoom. Ctrl/Cmd + wheel zooms at the pointer."
  }, /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": "Zoom out",
    disabled: zoom <= minZoom,
    onClick: () => setZoom(zoom / 1.5),
    style: {
      ...zb,
      opacity: zoom <= minZoom ? 0.4 : 1
    }
  }, "\u2212"), /*#__PURE__*/React.createElement("span", {
    style: {
      ...smValue,
      color: 'var(--text-secondary)',
      minWidth: 34,
      textAlign: 'center'
    }
  }, Math.round(zoom * 100), "%"), /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": "Zoom in",
    disabled: zoom >= maxZoom,
    onClick: () => setZoom(zoom * 1.5),
    style: {
      ...zb,
      opacity: zoom >= maxZoom ? 0.4 : 1
    }
  }, "+"), /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": "Zoom to fit",
    onClick: () => setZoom(1),
    style: {
      ...zb,
      width: 'auto',
      padding: '0 6px',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase'
    }
  }, "Fit"))), /*#__PURE__*/React.createElement("div", {
    ref: scrollRef,
    style: {
      overflowX: scroll ? 'auto' : 'hidden',
      overflowY: 'hidden',
      paddingBottom: scroll ? 4 : 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'grid',
      gridTemplateColumns: `${SM_AXIS}px ${W}px`,
      rowGap: 0
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'sticky',
      left: 0,
      zIndex: 2,
      height: H,
      background: 'var(--surface-panel)'
    }
  }, [127, 96, 64, 32, 1].map(v => /*#__PURE__*/React.createElement("span", {
    key: v,
    style: {
      position: 'absolute',
      right: 6,
      top: smClamp(yTop(v) - 6, 0, H - 12),
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-micro)',
      lineHeight: '12px',
      color: 'var(--text-tertiary)'
    }
  }, v))), /*#__PURE__*/React.createElement("div", {
    onPointerMove: e => setHover(gridPoint(e)),
    onPointerLeave: () => setHover(null),
    style: {
      position: 'relative',
      width: W,
      height: H,
      background: 'var(--dd-ink-1)',
      borderRadius: 'var(--radius-1, 2px)',
      overflow: 'hidden',
      boxShadow: 'inset 0 0 0 1px var(--border-hairline)',
      cursor: drag ? cursorFor(drag.mode) : 'default'
    }
  }, notes.map(n => /*#__PURE__*/React.createElement("div", {
    key: n,
    style: {
      position: 'absolute',
      left: xOf(n),
      top: 0,
      width: k,
      height: H,
      background: smIsBlack(n) ? 'var(--dd-ink-0)' : 'transparent',
      borderLeft: n % 12 === 0 ? '1px solid var(--dd-line-2)' : 'none',
      boxSizing: 'border-box',
      pointerEvents: 'none'
    }
  })), [96, 64, 32].map(v => /*#__PURE__*/React.createElement("div", {
    key: v,
    style: {
      position: 'absolute',
      left: 0,
      right: 0,
      top: yTop(v),
      height: 1,
      background: 'var(--dd-line-1)',
      pointerEvents: 'none'
    }
  })), (hover || held) && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: xOf((held || hover).note),
      top: 0,
      width: k,
      height: H,
      background: 'var(--dd-paper-1)',
      opacity: 0.06,
      pointerEvents: 'none'
    }
  }), ordered.map(z => {
    const isSel = z.id === selLocal,
      on = lit.includes(z.id);
    const si = stackIdx(z),
      sc = stackCount(z),
      off = Math.min(4, k / 5);
    const left = xOf(z.lo) + si * off,
      width = (z.hi - z.lo + 1) * k - (sc - 1) * off,
      top = yTop(z.velHi) + si * off,
      height = yBot(z.velLo) - yTop(z.velHi) - (sc - 1) * off;
    const e = Math.min(5, Math.max(2, width / 4)),
      ev = Math.min(5, Math.max(2, height / 4));
    const h = (m, s) => editable && /*#__PURE__*/React.createElement("div", {
      key: m,
      onPointerDown: ev2 => startDrag(ev2, z, m),
      style: {
        position: 'absolute',
        cursor: cursorFor(m),
        ...s
      }
    });
    return /*#__PURE__*/React.createElement("div", {
      key: z.id,
      role: "button",
      tabIndex: 0,
      "aria-label": `${z.name}${z.source ? ' (' + z.source + ')' : ''}, key ${range(z)}, velocity ${z.velLo} to ${z.velHi}`,
      "aria-pressed": isSel,
      onPointerDown: ev2 => startDrag(ev2, z, 'move'),
      onKeyDown: ev2 => onZoneKey(ev2, z),
      onFocus: () => {
        setFocusId(z.id);
        select(z.id);
      },
      onBlur: () => setFocusId(null),
      style: {
        position: 'absolute',
        left,
        top,
        width,
        height,
        boxSizing: 'border-box',
        outline: 'none',
        overflow: 'hidden',
        cursor: editable ? cursorFor('move') : 'pointer',
        background: isSel ? 'color-mix(in srgb, var(--dd-vermilion) 24%, transparent)' : 'color-mix(in srgb, var(--dd-ink-6) 72%, transparent)',
        border: `1px ${editable ? 'solid' : 'dashed'} ${isSel ? 'var(--dd-vermilion)' : 'var(--dd-line-3)'}`,
        borderRadius: 2,
        boxShadow: focusId === z.id ? '0 0 0 2px var(--dd-ink-1), 0 0 0 4px var(--color-focus)' : 'none'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        inset: 0,
        background: 'var(--dd-paper-1)',
        opacity: on ? 0.28 : 0,
        transition: on ? 'none' : 'opacity 400ms ease-out',
        pointerEvents: 'none'
      }
    }), width >= 34 && height >= 16 && (sc === 1 || isSel || !ordered.some(o => o !== z && sameRect(o, z) && o.id === selLocal) && si === sc - 1) && /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: 4,
        top: 2,
        right: 4,
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-micro)',
        fontWeight: 600,
        lineHeight: '12px',
        color: isSel ? 'var(--text-primary)' : 'var(--text-secondary)',
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        pointerEvents: 'none'
      }
    }, z.name, z.source && height >= 30 && /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'block',
        fontWeight: 500,
        color: 'var(--text-tertiary)'
      }
    }, z.source)), h('l', {
      left: 0,
      top: ev,
      bottom: ev,
      width: e
    }), h('r', {
      right: 0,
      top: ev,
      bottom: ev,
      width: e
    }), h('t', {
      top: 0,
      left: e,
      right: e,
      height: ev
    }), h('b', {
      bottom: 0,
      left: e,
      right: e,
      height: ev
    }), h('lt', {
      left: 0,
      top: 0,
      width: e,
      height: ev
    }), h('rt', {
      right: 0,
      top: 0,
      width: e,
      height: ev
    }), h('lb', {
      left: 0,
      bottom: 0,
      width: e,
      height: ev
    }), h('rb', {
      right: 0,
      bottom: 0,
      width: e,
      height: ev
    }));
  }), held && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: xOf(held.note) - 2,
      width: k + 4,
      top: yTop(held.vel),
      height: 2,
      background: 'var(--dd-paper-1)',
      pointerEvents: 'none'
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'sticky',
      left: 0,
      zIndex: 2,
      background: 'var(--surface-panel)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      width: W,
      height: 4,
      margin: '3px 0 2px'
    }
  }, notes.map(n => mapped(n) && /*#__PURE__*/React.createElement("div", {
    key: n,
    style: {
      position: 'absolute',
      left: xOf(n),
      width: k,
      top: 1,
      height: 2,
      background: inSel(n) ? 'var(--dd-vermilion)' : 'var(--dd-paper-4)',
      boxShadow: inSel(n) ? '0 -1px 0 var(--dd-vermilion), 0 1px 0 var(--dd-vermilion)' : 'none'
    }
  }))), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'sticky',
      left: 0,
      zIndex: 2,
      background: 'var(--surface-panel)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    onPointerUp: noteOff,
    onPointerCancel: noteOff,
    style: {
      position: 'relative',
      width: W,
      height: keyboardHeight,
      overflow: 'hidden',
      background: 'var(--dd-ink-0)',
      borderRadius: '0 0 2px 2px',
      touchAction: 'none'
    }
  }, whites.map(n => {
    const down = held && held.note === n,
      sel = inSel(n),
      root = selZone && selZone.root === n;
    return /*#__PURE__*/React.createElement("div", {
      key: n,
      role: "button",
      "aria-label": `Key ${n} ${noteName(n)}`,
      onPointerDown: e => noteOn(e, n),
      style: {
        position: 'absolute',
        left: whiteLeft(n),
        top: 0,
        width: whiteW,
        height: keyboardHeight,
        boxSizing: 'border-box',
        background: down ? 'var(--dd-vermilion)' : sel ? 'color-mix(in srgb, var(--dd-paper-2) 74%, var(--dd-vermilion))' : 'var(--dd-paper-2)',
        borderRight: '1px solid var(--dd-ink-2)',
        borderRadius: '0 0 2px 2px',
        cursor: 'pointer',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'flex-end',
        alignItems: 'center',
        gap: 3,
        paddingBottom: 3
      }
    }, root && /*#__PURE__*/React.createElement("span", {
      title: "Root key",
      style: {
        width: 5,
        height: 5,
        background: 'var(--dd-ink-0)',
        borderRadius: 1
      }
    }), n % 12 === 0 && whiteW >= 14 && /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-value)',
        fontSize: 'var(--type-micro)',
        lineHeight: 1,
        color: 'var(--dd-ink-3)',
        pointerEvents: 'none'
      }
    }, noteName(n)));
  }), notes.filter(smIsBlack).map(n => {
    const down = held && held.note === n,
      sel = inSel(n),
      root = selZone && selZone.root === n;
    return /*#__PURE__*/React.createElement("div", {
      key: n,
      role: "button",
      "aria-label": `Key ${n} ${noteName(n)}`,
      onPointerDown: e => noteOn(e, n),
      style: {
        position: 'absolute',
        left: xOf(n),
        top: 0,
        width: k,
        height: Math.round(keyboardHeight * 0.6),
        boxSizing: 'border-box',
        background: down ? 'var(--dd-vermilion)' : sel ? 'color-mix(in srgb, var(--dd-ink-1) 62%, var(--dd-vermilion))' : 'var(--dd-ink-1)',
        border: '1px solid var(--dd-ink-0)',
        borderTop: 'none',
        borderRadius: '0 0 2px 2px',
        cursor: 'pointer',
        display: 'flex',
        alignItems: 'flex-end',
        justifyContent: 'center',
        paddingBottom: 3
      }
    }, root && /*#__PURE__*/React.createElement("span", {
      title: "Root key",
      style: {
        width: 4,
        height: 4,
        background: 'var(--dd-paper-1)',
        borderRadius: 1
      }
    }));
  })))));
}
Object.assign(__ds_scope, { KeyMap });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/KeyMap.jsx", error: String((e && e.message) || e) }); }

// components/display/ModIndicator.jsx
try { (() => {
// Modulation slots pair colour + glyph + letter so meaning never relies on colour alone.
const MOD_SLOTS = {
  A: {
    color: 'var(--dd-mod-a)',
    glyph: 'circle',
    name: 'Mod A'
  },
  B: {
    color: 'var(--dd-mod-b)',
    glyph: 'triangle',
    name: 'Mod B'
  },
  C: {
    color: 'var(--dd-mod-c)',
    glyph: 'square',
    name: 'Mod C'
  },
  D: {
    color: 'var(--dd-mod-d)',
    glyph: 'diamond',
    name: 'Mod D'
  }
};
function ModGlyph({
  slot = 'A',
  size = 8,
  color,
  hollow = false
}) {
  const s = MOD_SLOTS[slot] || MOD_SLOTS.A;
  const c = color || s.color;
  const st = hollow ? {
    fill: 'none',
    stroke: c,
    strokeWidth: 1.5
  } : {
    fill: c
  };
  let shape;
  if (s.glyph === 'circle') shape = /*#__PURE__*/React.createElement("circle", {
    cx: "5",
    cy: "5",
    r: "3.6",
    style: st
  });else if (s.glyph === 'triangle') shape = /*#__PURE__*/React.createElement("path", {
    d: "M5 1.2 9 8.6H1Z",
    style: st
  });else if (s.glyph === 'square') shape = /*#__PURE__*/React.createElement("rect", {
    x: "1.6",
    y: "1.6",
    width: "6.8",
    height: "6.8",
    style: st
  });else shape = /*#__PURE__*/React.createElement("path", {
    d: "M5 .8 9.2 5 5 9.2.8 5Z",
    style: st
  });
  return /*#__PURE__*/React.createElement("svg", {
    width: size,
    height: size,
    viewBox: "0 0 10 10",
    style: {
      display: 'block',
      flex: 'none'
    },
    "aria-hidden": "true"
  }, shape);
}

/**
 * Compact chip naming a modulation source + depth. Used in tooltips, context menus,
 * and the pad-detail panel. Host = DAW-owned automation (blue, plug glyph, "HOST").
 */
function ModIndicator({
  slot = 'A',
  depth,
  source,
  host = false,
  active = false,
  compact = false
}) {
  const color = host ? 'var(--dd-host)' : (MOD_SLOTS[slot] || MOD_SLOTS.A).color;
  const depthText = depth == null ? null : `${depth > 0 ? '+' : depth < 0 ? '−' : ''}${Math.abs(Math.round(depth * 100))}%`;
  return /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 5,
      height: compact ? 18 : 20,
      padding: compact ? '0 5px' : '0 6px',
      borderRadius: 'var(--radius-1)',
      background: 'var(--dd-ink-0)',
      border: `1px solid ${active ? color : 'var(--border-control)'}`,
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      color: 'var(--text-secondary)',
      whiteSpace: 'nowrap',
      textTransform: 'uppercase'
    }
  }, host ? /*#__PURE__*/React.createElement("svg", {
    width: "9",
    height: "9",
    viewBox: "0 0 10 10",
    "aria-hidden": "true"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M3 1v3M7 1v3M1.5 4h7v1.5a3.5 3.5 0 0 1-7 0Z",
    style: {
      fill: 'none',
      stroke: color,
      strokeWidth: 1.4
    }
  })) : /*#__PURE__*/React.createElement(ModGlyph, {
    slot: slot,
    size: 9
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-primary)'
    }
  }, host ? 'Host' : source || slot), depthText && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      letterSpacing: 0,
      color
    }
  }, depthText));
}
Object.assign(__ds_scope, { MOD_SLOTS, ModGlyph, ModIndicator });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/ModIndicator.jsx", error: String((e && e.message) || e) }); }

// components/controls/Knob.jsx
try { (() => {
const SWEEP = 135; // degrees either side of 12 o'clock
const SIZES = {
  lg: 64,
  md: 48,
  sm: 36,
  xs: 28
};
function pt(c, r, deg) {
  const a = (deg - 90) * Math.PI / 180;
  return [c + r * Math.cos(a), c + r * Math.sin(a)];
}
function arc(c, r, d0, d1) {
  if (Math.abs(d1 - d0) < 0.01) return '';
  const a = Math.min(d0, d1),
    b = Math.max(d0, d1);
  const [x0, y0] = pt(c, r, a),
    [x1, y1] = pt(c, r, b);
  return `M${x0.toFixed(2)} ${y0.toFixed(2)}A${r} ${r} 0 ${b - a > 180 ? 1 : 0} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
}
const ang = v => -SWEEP + Math.max(0, Math.min(1, v)) * SWEEP * 2;
const clamp01 = v => Math.max(0, Math.min(1, v));

/**
 * Dandrum rotary knob. Anatomy (outside → in): modulation rings (≤2), value track + arc, graphite cap
 * carrying the combined-modulation arc. No pointer line — the value arc alone shows position.
 * All geometry is JUCE-drawable (Path::addCentredArc + fillEllipse).
 */
function Knob({
  value = 0.5,
  defaultValue = 0.5,
  bipolar = false,
  size = 'md',
  label,
  valueText,
  unit,
  modulations = [],
  assigning = false,
  hostAutomated = false,
  focused = false,
  disabled = false,
  hovered: hoveredProp,
  active: activeProp,
  activeSlot,
  peak: peakProp,
  trough: troughProp,
  peakText,
  troughText,
  showPeakTrough = true,
  onResetPeaks,
  valueDisplay = 'hover',
  parseValue,
  popupOpen,
  onChange,
  onContextMenu,
  onFocus,
  showLabel = true,
  labelPosition = 'top',
  style
}) {
  const px = typeof size === 'number' ? size : SIZES[size] || 48;
  const [hoverLocal, setHover] = React.useState(false);
  const [dragging, setDragging] = React.useState(false);
  const [focusLocal, setFocusLocal] = React.useState(false);
  const hovered = hoveredProp ?? hoverLocal;
  const isFocused = focused || focusLocal;
  const drag = React.useRef(null);
  // Value popup: values are hidden on the panel and appear in an editable popup on hover / focus / drag.
  const [popHover, setPopHover] = React.useState(false);
  const [editing, setEditing] = React.useState(false);
  const [draft, setDraft] = React.useState('');
  const closeT = React.useRef(null);
  const holdOpen = () => {
    clearTimeout(closeT.current);
    setPopHover(true);
  };
  const letClose = () => {
    clearTimeout(closeT.current);
    closeT.current = setTimeout(() => setPopHover(false), 250);
  };
  React.useEffect(() => () => clearTimeout(closeT.current), []);
  const [nudged, setNudged] = React.useState(false);
  const nudgeT = React.useRef(null);
  const nudge = () => {
    setNudged(true);
    clearTimeout(nudgeT.current);
    nudgeT.current = setTimeout(() => setNudged(false), 600);
  };
  React.useEffect(() => () => clearTimeout(nudgeT.current), []);
  // "Being changed" = user dragging / keyboard / wheel nudge, or the host writing the value.
  const tweaking = !disabled && (activeProp ?? (dragging || nudged || hostAutomated));
  const c = px / 2;
  // Layout reserves the thick widths so nothing shifts when arcs thicken.
  const trackMax = px >= 60 ? 4 : px >= 44 ? 3 : 2.5;
  const trackMin = px >= 44 ? 1.5 : 1.25;
  const modMax = px >= 44 ? 2 : 1.5;
  const modMin = 1;
  // Only the element being changed thickens: the value arc when the base value moves; a mod ring when its
  // source is live or its depth is being edited (activeSlot). The background track never thickens.
  const trackW = trackMin;
  const valueW = tweaking ? trackMax : trackMin;
  const rMod1 = c - modMax / 2;
  const rMod2 = rMod1 - modMax - 1;
  const rTrack = c - (px >= 44 ? 6.5 : 5) - trackMax / 2;
  const rCap = rTrack - trackMax / 2 - (px >= 44 ? 2.5 : 2);
  const rInner = rCap - (px >= 44 ? 3 : 2.5);
  const innerW = px >= 44 ? 2 : 1.5;
  const origin = bipolar ? 0 : -SWEEP;
  const vA = ang(value);
  const valueColor = disabled ? 'var(--dd-paper-4)' : hostAutomated ? 'var(--dd-host)' : 'var(--color-value)';
  const capFill = disabled ? 'var(--dd-ink-4)' : dragging ? 'var(--dd-cap-press, var(--dd-ink-6))' : hovered ? 'var(--dd-cap-hover)' : 'var(--dd-cap, var(--dd-ink-5))';
  const shownMods = modulations.slice(0, 2);
  // Combined modulation (all assignments): range = value + Σ positive / negative depths; live = value + Σ depth·live
  const sumPos = modulations.reduce((a, m) => a + Math.max(0, m.depth || 0), 0);
  const sumNeg = modulations.reduce((a, m) => a + Math.min(0, m.depth || 0), 0);
  const anyLive = modulations.some(m => m.live != null);
  const effective = value + modulations.reduce((a, m) => a + (m.depth || 0) * (m.live ?? 0), 0);
  // Peak / trough: extremes of the effective value since last reset (editor-held, decays on reset).
  const peakRef = React.useRef(null),
    troughRef = React.useRef(null);
  const [, bump] = React.useState(0);
  if (anyLive) {
    if (peakRef.current == null || effective > peakRef.current) peakRef.current = effective;
    if (troughRef.current == null || effective < troughRef.current) troughRef.current = effective;
  }
  const peak = peakProp ?? (modulations.length ? peakRef.current : null);
  const trough = troughProp ?? (modulations.length ? troughRef.current : null);
  const parse = parseValue || (t => {
    const n = parseFloat(String(t).replace('−', '-'));
    return Number.isFinite(n) ? n / 100 : null;
  });
  const commitDraft = () => {
    const v = parse(draft);
    if (v != null) setK(v);
    setEditing(false);
  };
  const resetPeaks = () => {
    peakRef.current = null;
    troughRef.current = null;
    bump(n => n + 1);
    onResetPeaks && onResetPeaks();
  };
  // Clip: modulation range reaches past the parameter range. Live clip = the effective value is pinned right now.
  const rangeClipHi = value + sumPos > 1.0001,
    rangeClipLo = value + sumNeg < -0.0001;
  const liveClipHi = anyLive && effective >= 1,
    liveClipLo = anyLive && effective <= 0;
  const clipWarn = !disabled && (rangeClipHi || rangeClipLo);
  const clipLive = !disabled && (liveClipHi || liveClipLo);
  const showInline = valueDisplay === 'always';
  const popOpen = !disabled && !showInline && (popupOpen ?? (popHover || editing || dragging || nudged || focusLocal));
  const combinedColor = modulations.length === 1 ? (__ds_scope.MOD_SLOTS[modulations[0].slot] || __ds_scope.MOD_SLOTS.A).color : 'var(--dd-paper-1)';
  const set = v => {
    if (!disabled && onChange) onChange(clamp01(v));
  };
  const setK = v => {
    nudge();
    set(v);
  };
  const onPointerDown = e => {
    if (disabled || e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    drag.current = {
      y: e.clientY,
      v: value
    };
    setDragging(true);
  };
  const onPointerMove = e => {
    if (!drag.current) return;
    const sens = e.shiftKey ? 800 : 200; // px for full range; Shift = fine
    set(drag.current.v + (drag.current.y - e.clientY) / sens);
  };
  const end = () => {
    drag.current = null;
    setDragging(false);
  };
  const onKeyDown = e => {
    const step = e.shiftKey ? 0.01 : 0.05;
    if (e.key === 'ArrowUp' || e.key === 'ArrowRight') {
      setK(value + step);
      e.preventDefault();
    } else if (e.key === 'ArrowDown' || e.key === 'ArrowLeft') {
      setK(value - step);
      e.preventDefault();
    } else if (e.key === 'Home') setK(0);else if (e.key === 'End') setK(1);else if (e.key === 'Delete' || e.key === 'Backspace') setK(defaultValue);else if (e.key === 'F10' && e.shiftKey || e.key === 'ContextMenu' || e.key === 'm' || e.key === 'M') {
      e.preventDefault();
      if (onContextMenu) {
        const r = e.currentTarget.getBoundingClientRect();
        onContextMenu({
          clientX: r.left + r.width / 2,
          clientY: r.bottom,
          preventDefault() {},
          fromKeyboard: true,
          target: e.currentTarget
        });
      }
    }
  };
  const labelEl = showLabel && label && /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: px <= 36 ? 'var(--type-micro)' : 'var(--type-label)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      lineHeight: 1.15,
      whiteSpace: 'nowrap',
      color: disabled ? 'var(--text-disabled)' : 'var(--text-secondary)',
      display: 'flex',
      alignItems: 'center',
      gap: 4
    }
  }, label, modulations.length > 2 && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: 10,
      color: 'var(--text-primary)',
      background: 'var(--dd-ink-0)',
      borderRadius: 2,
      padding: '0 3px',
      letterSpacing: 0
    }
  }, "+", modulations.length - 2));
  return /*#__PURE__*/React.createElement("div", {
    role: "slider",
    tabIndex: disabled ? -1 : 0,
    "aria-label": label,
    "aria-valuemin": 0,
    "aria-valuemax": 1,
    "aria-valuenow": Number(value.toFixed(3)),
    "aria-valuetext": valueText,
    "aria-disabled": disabled || undefined,
    onPointerDown: onPointerDown,
    onPointerMove: onPointerMove,
    onPointerUp: end,
    onPointerCancel: end,
    onDoubleClick: () => setK(defaultValue),
    onKeyDown: e => {
      if (e.key === 'Enter' && !disabled && !showInline) {
        e.preventDefault();
        setDraft(String(valueText ?? Math.round(value * 100)));
        setEditing(true);
        return;
      }
      onKeyDown(e);
    },
    onWheel: e => {
      if (disabled) return;
      setK(value + (e.deltaY < 0 ? 1 : -1) * (e.shiftKey ? 0.01 : 0.02));
    },
    onContextMenu: e => {
      e.preventDefault();
      if (!disabled && onContextMenu) onContextMenu(e);
    },
    onMouseEnter: () => {
      setHover(true);
      holdOpen();
    },
    onMouseLeave: () => {
      setHover(false);
      letClose();
    },
    onFocus: e => {
      setFocusLocal(true);
      onFocus && onFocus(e);
    },
    onBlur: () => setFocusLocal(false),
    style: {
      display: 'inline-flex',
      flexDirection: labelPosition === 'top' ? 'column' : 'column-reverse',
      alignItems: 'center',
      gap: px <= 36 ? 3 : 5,
      padding: px <= 36 ? '4px 4px' : '6px 6px',
      borderRadius: 'var(--radius-2)',
      outline: 'none',
      cursor: disabled ? 'default' : 'ns-resize',
      userSelect: 'none',
      touchAction: 'none',
      position: 'relative',
      minWidth: px + 12,
      background: assigning && !disabled ? 'var(--dd-mod-wash)' : 'transparent',
      boxShadow: isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : 'none',
      ...style
    }
  }, labelEl, /*#__PURE__*/React.createElement("svg", {
    width: px,
    height: px,
    viewBox: `0 0 ${px} ${px}`,
    style: {
      display: 'block',
      overflow: 'visible'
    },
    "aria-hidden": "true"
  }, assigning && !disabled && /*#__PURE__*/React.createElement("circle", {
    cx: c,
    cy: c,
    r: rMod1,
    style: {
      fill: 'none',
      stroke: 'var(--dd-mod-a)',
      strokeWidth: 1.5,
      strokeDasharray: '3 3',
      opacity: 0.9
    }
  }), !disabled && shownMods.map((m, i) => {
    const r = i === 0 ? rMod1 : rMod2;
    const col = (__ds_scope.MOD_SLOTS[m.slot] || __ds_scope.MOD_SLOTS.A).color;
    const end = ang(value + (m.depth || 0));
    const live = m.live != null;
    const changing = live || m.slot === activeSlot;
    const w = changing ? modMax : modMin;
    return /*#__PURE__*/React.createElement("g", {
      key: i
    }, /*#__PURE__*/React.createElement("path", {
      d: arc(c, r, -SWEEP, SWEEP),
      style: {
        fill: 'none',
        stroke: 'var(--dd-ink-3)',
        strokeWidth: modMin
      }
    }), /*#__PURE__*/React.createElement("path", {
      d: arc(c, r, vA, end),
      style: {
        fill: 'none',
        stroke: col,
        strokeWidth: w,
        strokeLinecap: 'butt',
        opacity: changing ? 1 : 0.7
      }
    }), live && (() => {
      const [x, y] = pt(c, r, ang(value + (m.depth || 0) * m.live));
      return /*#__PURE__*/React.createElement("circle", {
        cx: x,
        cy: y,
        r: modMax / 2 + 1.2,
        style: {
          fill: col,
          stroke: 'var(--surface-panel)',
          strokeWidth: 1
        }
      });
    })());
  }), /*#__PURE__*/React.createElement("path", {
    d: arc(c, rTrack, -SWEEP, SWEEP),
    style: {
      fill: 'none',
      stroke: disabled ? 'var(--dd-ink-4)' : 'var(--color-track)',
      strokeWidth: trackW,
      strokeLinecap: 'round'
    }
  }), !disabled && /*#__PURE__*/React.createElement("path", {
    d: arc(c, rTrack, origin, vA),
    style: {
      fill: 'none',
      stroke: valueColor,
      strokeWidth: valueW,
      strokeLinecap: 'round'
    }
  }), (() => {
    const [x0, y0] = pt(c, rTrack + trackW / 2 + 1, origin);
    const [x1, y1] = pt(c, rTrack + trackW / 2 + (px >= 44 ? 4 : 3), origin);
    return /*#__PURE__*/React.createElement("line", {
      x1: x0,
      y1: y0,
      x2: x1,
      y2: y1,
      style: {
        stroke: 'var(--dd-paper-3)',
        strokeWidth: 1.5,
        strokeLinecap: 'round',
        opacity: disabled ? 0.4 : 1
      }
    });
  })(), /*#__PURE__*/React.createElement("circle", {
    cx: c,
    cy: c + 1.5,
    r: rCap,
    style: {
      fill: 'rgba(0,0,0,0.45)'
    }
  }), /*#__PURE__*/React.createElement("circle", {
    cx: c,
    cy: c,
    r: rCap,
    style: {
      fill: capFill,
      stroke: 'var(--dd-line-3)',
      strokeWidth: 1,
      strokeOpacity: disabled ? 0.3 : 0.6
    }
  }), /*#__PURE__*/React.createElement("path", {
    d: arc(c, rCap - 1, -60, 60),
    style: {
      fill: 'none',
      stroke: 'rgba(255,255,255,0.09)',
      strokeWidth: 1
    }
  }), !disabled && modulations.length > 0 && /*#__PURE__*/React.createElement("g", null, /*#__PURE__*/React.createElement("path", {
    d: arc(c, rInner, ang(value + sumNeg), ang(value + sumPos)),
    style: {
      fill: 'none',
      stroke: combinedColor,
      strokeWidth: innerW,
      strokeLinecap: 'butt',
      opacity: 0.3
    }
  }), anyLive && /*#__PURE__*/React.createElement("path", {
    d: arc(c, rInner, vA, ang(effective)),
    style: {
      fill: 'none',
      stroke: combinedColor,
      strokeWidth: innerW,
      strokeLinecap: 'butt'
    }
  }), anyLive && (() => {
    const [x, y] = pt(c, rInner, ang(effective));
    return /*#__PURE__*/React.createElement("circle", {
      cx: x,
      cy: y,
      r: innerW / 2 + 1,
      style: {
        fill: combinedColor
      }
    });
  })(), showPeakTrough && [[peak, 1], [trough, -1]].map(([p, dir], i) => {
    if (p == null) return null;
    const a = ang(p),
      [x0, y0] = pt(c, rInner + dir * (innerW / 2 + 0.5), a),
      [x1, y1] = pt(c, rInner + dir * (innerW / 2 + (px >= 44 ? 3 : 2.5)), a);
    return /*#__PURE__*/React.createElement("line", {
      key: i,
      x1: x0,
      y1: y0,
      x2: x1,
      y2: y1,
      style: {
        stroke: 'var(--dd-paper-1)',
        strokeWidth: 1.25,
        strokeLinecap: 'round',
        opacity: 0.85
      }
    });
  })), clipWarn && [[rangeClipHi, SWEEP, liveClipHi], [rangeClipLo, -SWEEP, liveClipLo]].map(([on, a, live], i) => {
    if (!on) return null;
    const [x, y] = pt(c, rTrack, a);
    return /*#__PURE__*/React.createElement("circle", {
      key: 'clip' + i,
      cx: x,
      cy: y,
      r: trackMax / 2 + 1.5,
      style: live ? {
        fill: 'var(--dd-error)'
      } : {
        fill: 'var(--surface-panel)',
        stroke: 'var(--dd-error)',
        strokeWidth: 1.5
      }
    });
  })), showInline ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: px >= 60 ? 'var(--type-value-lg)' : px <= 36 ? 'var(--type-micro)' : 'var(--type-value)',
      fontWeight: 500,
      lineHeight: 1.1,
      whiteSpace: 'nowrap',
      minHeight: '1.1em',
      display: 'flex',
      alignItems: 'center',
      gap: 4,
      color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)'
    }
  }, shownMods.length > 0 && !disabled && /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 2
    }
  }, shownMods.map((m, i) => /*#__PURE__*/React.createElement(__ds_scope.ModGlyph, {
    key: i,
    slot: m.slot,
    size: 7
  }))), /*#__PURE__*/React.createElement("span", null, valueText ?? Math.round(value * 100)), unit && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-tertiary)'
    }
  }, unit), clipWarn && /*#__PURE__*/React.createElement("span", {
    title: clipLive ? 'Modulation is clipping at the range limit' : 'Modulation range exceeds the parameter range',
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 2,
      height: 14,
      padding: '0 3px',
      borderRadius: 2,
      fontFamily: 'var(--font-ui)',
      fontSize: 10,
      fontWeight: 700,
      letterSpacing: '0.06em',
      background: clipLive ? 'var(--dd-error)' : 'transparent',
      color: clipLive ? 'var(--dd-ink-0)' : 'var(--dd-error)',
      boxShadow: clipLive ? 'none' : 'inset 0 0 0 1px var(--dd-error)'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "8",
    height: "8",
    viewBox: "0 0 10 10",
    "aria-hidden": "true"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M5 1 9.3 9H.7Z",
    style: {
      fill: 'none',
      stroke: 'currentColor',
      strokeWidth: 1.5,
      strokeLinejoin: 'round'
    }
  })), "CLIP")), showPeakTrough && !disabled && peak != null && trough != null && px >= 44 && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: e => {
      e.stopPropagation();
      resetPeaks();
    },
    onPointerDown: e => e.stopPropagation(),
    title: "Peak / trough of the modulated value \xB7 click to reset",
    style: {
      display: 'flex',
      gap: 6,
      padding: 0,
      border: 0,
      background: 'transparent',
      cursor: 'pointer',
      fontFamily: 'var(--font-value)',
      fontSize: 10,
      lineHeight: 1,
      color: 'var(--text-tertiary)',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement("span", null, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, "\u25B4"), peakText ?? Math.round(clamp01(peak) * 100)), /*#__PURE__*/React.createElement("span", null, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, "\u25BE"), troughText ?? Math.round(clamp01(trough) * 100)))) : /*#__PURE__*/React.createElement("div", {
    style: {
      height: 14,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, clipWarn && /*#__PURE__*/React.createElement("span", {
    title: clipLive ? 'Modulation is clipping at the range limit' : 'Modulation range exceeds the parameter range',
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 2,
      height: 14,
      padding: '0 3px',
      borderRadius: 2,
      fontFamily: 'var(--font-ui)',
      fontSize: 10,
      fontWeight: 700,
      letterSpacing: '0.06em',
      flex: 'none',
      background: clipLive ? 'var(--dd-error)' : 'transparent',
      color: clipLive ? 'var(--dd-ink-0)' : 'var(--dd-error)',
      boxShadow: clipLive ? 'none' : 'inset 0 0 0 1px var(--dd-error)'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "8",
    height: "8",
    viewBox: "0 0 10 10",
    "aria-hidden": "true"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M5 1 9.3 9H.7Z",
    style: {
      fill: 'none',
      stroke: 'currentColor',
      strokeWidth: 1.5,
      strokeLinejoin: 'round'
    }
  })), "CLIP")), popOpen && /*#__PURE__*/React.createElement("div", {
    role: "dialog",
    "aria-label": (label || 'Value') + ' value',
    onMouseEnter: holdOpen,
    onMouseLeave: letClose,
    onPointerDown: e => e.stopPropagation(),
    onDoubleClick: e => e.stopPropagation(),
    onWheel: e => e.stopPropagation(),
    onKeyDown: e => e.stopPropagation(),
    style: {
      position: 'absolute',
      top: '100%',
      left: '50%',
      transform: 'translate(-50%, -2px)',
      zIndex: 30,
      minWidth: Math.max(104, px + 32),
      display: 'flex',
      flexDirection: 'column',
      gap: 5,
      padding: 6,
      boxSizing: 'border-box',
      cursor: 'default',
      background: 'var(--dd-ink-0)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--radius-2)',
      boxShadow: 'var(--shadow-float)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      top: -5,
      left: '50%',
      marginLeft: -4,
      width: 8,
      height: 8,
      transform: 'rotate(45deg)',
      background: 'var(--dd-ink-0)',
      borderLeft: '1px solid var(--border-strong)',
      borderTop: '1px solid var(--border-strong)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    onClick: () => {
      if (!editing) {
        setDraft(String(valueText ?? Math.round(value * 100)));
        setEditing(true);
      }
    },
    title: "Click to type a value",
    style: {
      position: 'relative',
      display: 'flex',
      alignItems: 'center',
      gap: 4,
      height: 24,
      padding: '0 6px',
      borderRadius: 'var(--radius-1)',
      cursor: 'text',
      background: 'var(--dd-ink-2)',
      border: `1px solid ${editing ? 'var(--dd-paper-2)' : 'var(--border-control)'}`,
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-value)',
      color: hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)',
      whiteSpace: 'nowrap'
    }
  }, editing ? /*#__PURE__*/React.createElement("input", {
    autoFocus: true,
    value: draft,
    onChange: e => setDraft(e.target.value),
    onBlur: commitDraft,
    onKeyDown: e => {
      if (e.key === 'Enter') commitDraft();
      if (e.key === 'Escape') setEditing(false);
      e.stopPropagation();
    },
    onFocus: e => e.target.select(),
    style: {
      flex: 1,
      minWidth: 0,
      width: 56,
      background: 'transparent',
      border: 0,
      outline: 'none',
      padding: 0,
      color: 'var(--text-primary)',
      font: 'inherit'
    }
  }) : /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }, valueText ?? Math.round(value * 100)), unit && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-tertiary)',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)'
    }
  }, unit)), (clipWarn || showPeakTrough && peak != null && trough != null) && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, showPeakTrough && peak != null && trough != null && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: resetPeaks,
    title: "Peak / trough of the modulated value \xB7 click to reset",
    style: {
      display: 'flex',
      gap: 6,
      padding: 0,
      border: 0,
      background: 'transparent',
      cursor: 'pointer',
      fontFamily: 'var(--font-value)',
      fontSize: 10,
      lineHeight: 1,
      color: 'var(--text-tertiary)',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement("span", null, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, "\u25B4"), peakText ?? Math.round(clamp01(peak) * 100)), /*#__PURE__*/React.createElement("span", null, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-secondary)'
    }
  }, "\u25BE"), troughText ?? Math.round(clamp01(trough) * 100))), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }), clipWarn && /*#__PURE__*/React.createElement("span", {
    title: clipLive ? 'Modulation is clipping at the range limit' : 'Modulation range exceeds the parameter range',
    style: {
      display: 'inline-flex',
      alignItems: 'center',
      gap: 2,
      height: 14,
      padding: '0 3px',
      borderRadius: 2,
      fontFamily: 'var(--font-ui)',
      fontSize: 10,
      fontWeight: 700,
      letterSpacing: '0.06em',
      flex: 'none',
      background: clipLive ? 'var(--dd-error)' : 'transparent',
      color: clipLive ? 'var(--dd-ink-0)' : 'var(--dd-error)',
      boxShadow: clipLive ? 'none' : 'inset 0 0 0 1px var(--dd-error)'
    }
  }, /*#__PURE__*/React.createElement("svg", {
    width: "8",
    height: "8",
    viewBox: "0 0 10 10",
    "aria-hidden": "true"
  }, /*#__PURE__*/React.createElement("path", {
    d: "M5 1 9.3 9H.7Z",
    style: {
      fill: 'none',
      stroke: 'currentColor',
      strokeWidth: 1.5,
      strokeLinejoin: 'round'
    }
  })), "CLIP")), modulations.length > 0 && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 2,
      paddingTop: 4,
      borderTop: '1px solid var(--border-hairline)'
    }
  }, modulations.map((m, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 5,
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      color: 'var(--text-secondary)',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.ModGlyph, {
    slot: m.slot,
    size: 8
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1
    }
  }, m.source || (__ds_scope.MOD_SLOTS[m.slot] || __ds_scope.MOD_SLOTS.A).name), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      color: (__ds_scope.MOD_SLOTS[m.slot] || __ds_scope.MOD_SLOTS.A).color
    }
  }, (m.depth > 0 ? '+' : m.depth < 0 ? '−' : '') + Math.abs(Math.round((m.depth || 0) * 100)), "%")))), hostAutomated && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      color: 'var(--dd-host)'
    }
  }, "Host automation")));
}
Object.assign(__ds_scope, { Knob });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/Knob.jsx", error: String((e && e.message) || e) }); }

// components/controls/Slider.jsx
try { (() => {
const clamp01 = v => Math.max(0, Math.min(1, v));

/**
 * Linear slider, horizontal or vertical. Track 4px, thumb 12px square-ish (radius 2).
 * Modulation range = thin bar(s) beside the track in the slot colour; host automation tints fill blue.
 */
function Slider({
  value = 0.5,
  defaultValue = 0.5,
  orientation = 'horizontal',
  length,
  bipolar = false,
  label,
  valueText,
  modulations = [],
  assigning = false,
  hostAutomated = false,
  focused = false,
  disabled = false,
  compact = false,
  onChange,
  onContextMenu,
  style
}) {
  const vertical = orientation === 'vertical';
  const L = length ?? (vertical ? compact ? 88 : 120 : compact ? 112 : 160);
  const thick = compact ? 20 : 24;
  const [focusLocal, setFocusLocal] = React.useState(false);
  const [hover, setHover] = React.useState(false);
  const [dragging, setDragging] = React.useState(false);
  const ref = React.useRef(null);
  const thumb = 12;
  const usable = L - thumb;
  const pos = v => thumb / 2 + clamp01(v) * usable;
  const set = v => {
    if (!disabled && onChange) onChange(clamp01(v));
  };
  const fromEvent = e => {
    const r = ref.current.getBoundingClientRect();
    return vertical ? 1 - (e.clientY - r.top - thumb / 2) / usable : (e.clientX - r.left - thumb / 2) / usable;
  };
  const onPointerDown = e => {
    if (disabled || e.button !== 0) return;
    e.currentTarget.setPointerCapture(e.pointerId);
    setDragging(true);
    set(fromEvent(e));
  };
  const onPointerMove = e => {
    if (dragging) set(fromEvent(e));
  };
  const end = () => setDragging(false);
  const onKeyDown = e => {
    const step = e.shiftKey ? 0.01 : 0.05;
    if (['ArrowUp', 'ArrowRight'].includes(e.key)) {
      set(value + step);
      e.preventDefault();
    } else if (['ArrowDown', 'ArrowLeft'].includes(e.key)) {
      set(value - step);
      e.preventDefault();
    } else if (e.key === 'Delete' || e.key === 'Backspace') set(defaultValue);else if (e.key === 'F10' && e.shiftKey || e.key === 'ContextMenu' || e.key === 'm' || e.key === 'M') {
      e.preventDefault();
      if (onContextMenu) {
        const r = e.currentTarget.getBoundingClientRect();
        onContextMenu({
          clientX: r.left,
          clientY: r.bottom,
          preventDefault() {},
          fromKeyboard: true
        });
      }
    }
  };
  const isFocused = focused || focusLocal;
  const fillColor = disabled ? 'var(--dd-ink-6)' : hostAutomated ? 'var(--dd-host)' : 'var(--color-value)';
  const o = bipolar ? pos(0.5) : pos(0);
  const p = pos(value);
  const lo = Math.min(o, p),
    hi = Math.max(o, p);
  const mid = thick / 2;
  const seg = (a, b, off, w, color, opacity = 1, key) => vertical ? /*#__PURE__*/React.createElement("div", {
    key: key,
    style: {
      position: 'absolute',
      left: mid + off - w / 2,
      width: w,
      bottom: Math.min(a, b),
      height: Math.abs(b - a),
      background: color,
      opacity,
      borderRadius: 1
    }
  }) : /*#__PURE__*/React.createElement("div", {
    key: key,
    style: {
      position: 'absolute',
      top: mid + off - w / 2,
      height: w,
      left: Math.min(a, b),
      width: Math.abs(b - a),
      background: color,
      opacity,
      borderRadius: 1
    }
  });
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'inline-flex',
      flexDirection: vertical ? 'column' : 'column',
      alignItems: vertical ? 'center' : 'stretch',
      gap: 4,
      ...style
    }
  }, label && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      justifyContent: 'space-between',
      gap: 8,
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-micro)' : 'var(--type-label)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: disabled ? 'var(--text-disabled)' : 'var(--text-secondary)',
      whiteSpace: 'nowrap'
    }
  }, /*#__PURE__*/React.createElement("span", null, label), !vertical && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      textTransform: 'none',
      letterSpacing: 0,
      fontWeight: 500,
      fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)',
      color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)'
    }
  }, valueText ?? Math.round(value * 100))), /*#__PURE__*/React.createElement("div", {
    ref: ref,
    role: "slider",
    tabIndex: disabled ? -1 : 0,
    "aria-label": label,
    "aria-orientation": orientation,
    "aria-valuemin": 0,
    "aria-valuemax": 1,
    "aria-valuenow": Number(value.toFixed(3)),
    "aria-valuetext": valueText,
    onPointerDown: onPointerDown,
    onPointerMove: onPointerMove,
    onPointerUp: end,
    onPointerCancel: end,
    onDoubleClick: () => set(defaultValue),
    onKeyDown: onKeyDown,
    onContextMenu: e => {
      e.preventDefault();
      if (!disabled && onContextMenu) onContextMenu(e);
    },
    onFocus: () => setFocusLocal(true),
    onBlur: () => setFocusLocal(false),
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    style: {
      position: 'relative',
      width: vertical ? thick : L,
      height: vertical ? L : thick,
      outline: 'none',
      touchAction: 'none',
      cursor: disabled ? 'default' : vertical ? 'ns-resize' : 'ew-resize',
      borderRadius: 'var(--radius-1)',
      background: assigning && !disabled ? 'var(--dd-mod-wash)' : 'transparent',
      boxShadow: isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : assigning && !disabled ? 'inset 0 0 0 1px var(--dd-mod-a)' : 'none'
    }
  }, seg(thumb / 2, L - thumb / 2, 0, 4, disabled ? 'var(--dd-ink-4)' : 'var(--color-track)'), !disabled && seg(lo, hi, 0, 4, fillColor), bipolar && seg(o - 0.75, o + 0.75, 0, 10, 'var(--dd-paper-3)'), !disabled && modulations.slice(0, 2).map((m, i) => {
    const col = (__ds_scope.MOD_SLOTS[m.slot] || __ds_scope.MOD_SLOTS.A).color;
    const e = pos(value + (m.depth || 0));
    const off = (i === 0 ? -1 : 1) * 6.5;
    const nodes = [seg(p, e, off, 2, col, m.live != null ? 1 : 0.6, 'r' + i)];
    if (m.live != null) {
      const lp = pos(value + (m.depth || 0) * m.live);
      nodes.push(seg(lp - 1.5, lp + 1.5, off, 4, col, 1, 'l' + i));
    }
    return nodes;
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      width: vertical ? 16 : thumb,
      height: vertical ? thumb : 16,
      borderRadius: 2,
      left: vertical ? mid - 8 : p - thumb / 2,
      top: vertical ? undefined : mid - 8,
      bottom: vertical ? p - thumb / 2 : undefined,
      background: disabled ? 'var(--dd-ink-4)' : dragging ? 'var(--dd-cap-press, var(--dd-ink-6))' : hover ? 'var(--dd-cap-hover)' : 'var(--dd-cap, var(--dd-ink-5))',
      border: '1px solid var(--dd-line-3)',
      boxShadow: disabled ? 'none' : 'var(--shadow-cap), var(--highlight-cap)',
      boxSizing: 'border-box',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      width: vertical ? 8 : 2,
      height: vertical ? 2 : 8,
      background: disabled ? 'var(--dd-paper-4)' : 'var(--dd-paper-1)',
      borderRadius: 1
    }
  }))), vertical && /*#__PURE__*/React.createElement("div", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)',
      color: disabled ? 'var(--text-disabled)' : hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)',
      textAlign: 'center'
    }
  }, valueText ?? Math.round(value * 100)));
}
Object.assign(__ds_scope, { Slider });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/Slider.jsx", error: String((e && e.message) || e) }); }

// components/display/WaveformPanel.jsx
try { (() => {
// Deterministic placeholder audio so mockups render the same every time. JUCE draws real
// min/max peaks from the prepared AudioThumbnail; nothing here is baked into an asset.
function rng(seed) {
  let s = seed >>> 0;
  return () => {
    s = s * 1664525 + 1013904223 >>> 0;
    return s / 4294967296;
  };
}
const SHAPES = {
  kick: (t, r) => Math.exp(-t * 5.5) * (0.65 + 0.35 * Math.abs(Math.sin(t * 60 * (1 - t * 0.6)))) + r() * 0.04 * Math.exp(-t * 30),
  snare: (t, r) => Math.exp(-t * 7) * (0.35 + 0.65 * r()) + Math.exp(-t * 25) * 0.4,
  'snare-soft': (t, r) => 0.55 * (Math.exp(-t * 8) * (0.35 + 0.65 * r()) + Math.exp(-t * 25) * 0.3),
  'hat-closed': (t, r) => Math.exp(-t * 18) * (0.3 + 0.7 * r()),
  'hat-open': (t, r) => Math.exp(-t * 3.2) * (0.25 + 0.6 * r()) * (t < 0.01 ? t * 100 : 1),
  break: (t, r) => {
    const hits = [0, .125, .25, .3125, .5, .625, .75, .875];
    let a = 0.06 * r();
    for (const h of hits) {
      if (t >= h) a = Math.max(a, Math.exp(-(t - h) * (h % .25 === 0 ? 22 : 40)) * (h % .5 === 0 ? 0.95 : 0.6) * (0.5 + 0.5 * r()));
    }
    return a;
  }
};
function makePeaks(kind = 'kick', n = 400, seed = 7) {
  const r = rng(seed + kind.length * 31),
    f = SHAPES[kind] || SHAPES.kick,
    out = [];
  for (let i = 0; i < n; i++) out.push(Math.min(1, f(i / n, r)));
  return out;
}
const pct = v => `${(v * 100).toFixed(3)}%`;

// Spectral energy per frequency bin (0 = low, 1 = high) for each placeholder sound.
const SPECTRA = {
  kick: (f, t) => Math.exp(-f * 9) * (1 + 0.6 * Math.exp(-t * 40)) + 0.25 * Math.exp(-t * 60) * Math.exp(-f * 2),
  snare: (f, t) => 0.55 * Math.exp(-Math.pow((f - 0.12) * 9, 2)) + 0.5 * Math.exp(-f * 1.4) * Math.exp(-t * 4),
  'snare-soft': (f, t) => 0.5 * Math.exp(-Math.pow((f - 0.12) * 9, 2)) + 0.35 * Math.exp(-f * 2) * Math.exp(-t * 5),
  'hat-closed': f => Math.pow(f, 1.4) * 0.9 + 0.05,
  'hat-open': f => Math.pow(f, 1.2) * 0.85 + 0.06,
  break: (f, t, r) => {
    const lo = Math.exp(-f * 8),
      hi = Math.pow(f, 1.3);
    const k = r < 0.5 ? lo : 0.6 * lo + 0.6 * hi;
    return k;
  }
};
function spectralColor(v, ramp) {
  const x = Math.max(0, Math.min(1, v)) * (ramp.length - 1),
    i = Math.floor(x),
    t = x - i;
  const a = ramp[i],
    b = ramp[Math.min(ramp.length - 1, i + 1)];
  return `rgb(${a.map((c, k) => Math.round(c + (b[k] - c) * t)).join(',')})`;
}
const hex = h => [1, 3, 5].map(i => parseInt(h.slice(i, i + 2), 16));

/** Spectrogram canvas. JUCE: compute once at patch load (FFT 1024, hop 256, log-frequency) into a cached Image. */
function Spectrogram({
  data,
  kind,
  reversed,
  regionStart,
  regionEnd
}) {
  const ref = React.useRef(null);
  React.useEffect(() => {
    const cv = ref.current;
    if (!cv) return;
    const cs = getComputedStyle(cv);
    const v = (n, d) => cs.getPropertyValue(n).trim() || d;
    const ramp = [v('--dd-ink-0', '#130F0C'), v('--dd-ink-5', '#41362C'), v('--dd-vermilion-lo', '#B0662F'), v('--dd-vermilion', '#E08A4E'), v('--dd-paper-1', '#F2E6D3')].map(hex);
    const cols = data.length,
      rows = 64;
    cv.width = cols;
    cv.height = rows;
    const ctx = cv.getContext('2d');
    const prof = SPECTRA[kind] || SPECTRA.kick;
    let s = 99;
    const rnd = () => {
      s = s * 1664525 + 1013904223 >>> 0;
      return s / 4294967296;
    };
    for (let x = 0; x < cols; x++) {
      const i = reversed ? cols - 1 - x : x;
      const t = i / cols,
        amp = data[i];
      const hitR = rnd();
      for (let y = 0; y < rows; y++) {
        const f = 1 - y / (rows - 1);
        const e = amp * prof(f, t, hitR) * (0.75 + 0.5 * rnd());
        const db = Math.max(0, 1 + Math.log10(Math.max(1e-4, e)) / 2.2);
        const inRegion = x / cols >= regionStart && x / cols <= regionEnd;
        ctx.fillStyle = spectralColor(inRegion ? db : db * 0.45, ramp);
        ctx.fillRect(x, y, 1, 1);
      }
    }
  }, [data, kind, reversed, regionStart, regionEnd]);
  return /*#__PURE__*/React.createElement("canvas", {
    ref: ref,
    style: {
      position: 'absolute',
      left: 0,
      top: 18,
      width: '100%',
      height: 'calc(100% - 18px)',
      imageRendering: 'auto'
    },
    "aria-hidden": "true"
  });
}
function ViewToggle({
  value,
  onChange
}) {
  const opt = (id, label) => /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-pressed": value === id,
    onClick: () => onChange(id),
    style: {
      height: 14,
      padding: '0 5px',
      border: 0,
      borderRadius: 2,
      cursor: 'pointer',
      fontFamily: 'var(--font-ui)',
      fontSize: 10,
      fontWeight: 600,
      letterSpacing: '0.06em',
      textTransform: 'uppercase',
      background: value === id ? 'var(--dd-ink-5)' : 'transparent',
      color: value === id ? 'var(--text-primary)' : 'var(--text-tertiary)'
    }
  }, label);
  return /*#__PURE__*/React.createElement("div", {
    role: "radiogroup",
    "aria-label": "Display",
    style: {
      position: 'absolute',
      bottom: 6,
      right: 6,
      display: 'flex',
      gap: 1,
      padding: 1,
      borderRadius: 3,
      background: 'var(--dd-ink-0)',
      border: '1px solid var(--border-hairline)',
      zIndex: 2
    }
  }, opt('wave', 'Wave'), opt('spectral', 'Spectral'));
}

/**
 * Waveform panel for the selected sample region. Everything shown is prepared data except
 * `cursor` (playback position) and `startOffset` (live parameter). Not an editor: markers are not draggable.
 */
function WaveformPanel({
  kind = 'kick',
  peaks,
  height = 200,
  regionStart = 0,
  regionEnd = 1,
  fadeIn = 0,
  fadeOut = 0,
  loopStart,
  loopEnd,
  crossfade = 0,
  slices,
  selectedSlice,
  cursor,
  startOffset = 0,
  reversed = false,
  hostAutomated = false,
  startModulation,
  compact = false,
  missing = false,
  label,
  style,
  display: displayProp,
  defaultDisplay = 'wave',
  onDisplayChange,
  showDisplayToggle = true
}) {
  const [displayLocal, setDisplayLocal] = React.useState(defaultDisplay);
  const display = displayProp ?? displayLocal;
  const setDisplay = d => {
    if (displayProp == null) setDisplayLocal(d);
    onDisplayChange && onDisplayChange(d);
  };
  const spectral = display === 'spectral';
  // slices: numbers (positions) or { pos, name } — names are prepared patch data
  const sliceList = slices ? slices.map(x => typeof x === 'number' ? {
    pos: x
  } : x) : null;
  const data = React.useMemo(() => peaks || makePeaks(kind, compact ? 240 : 400), [peaks, kind, compact]);
  const N = data.length;
  const shown = reversed ? data.map((_, i) => data[i]) : data;
  const W = 1000,
    H = 100,
    mid = H / 2;
  const path = React.useMemo(() => {
    const src = reversed ? [...shown].reverse() : shown;
    let top = `M0 ${mid}`,
      bot = '';
    src.forEach((p, i) => {
      const x = i / (N - 1) * W;
      const a = p * (mid - 4);
      top += `L${x.toFixed(1)} ${(mid - a).toFixed(1)}`;
      bot = `L${x.toFixed(1)} ${(mid + a).toFixed(1)}` + bot;
    });
    return top + `L${W} ${mid}` + bot.replace(/^L/, 'L') + 'Z';
  }, [shown, reversed, N]);
  const regionW = regionEnd - regionStart;
  const startPos = regionStart + startOffset * regionW;
  const flag = {
    position: 'absolute',
    top: 0,
    height: 16,
    padding: '0 4px',
    display: 'flex',
    alignItems: 'center',
    fontFamily: 'var(--font-value)',
    fontSize: 10,
    lineHeight: 1,
    borderRadius: '0 2px 2px 0',
    whiteSpace: 'nowrap'
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      height,
      background: 'var(--surface-well)',
      borderRadius: 'var(--radius-1)',
      overflow: 'hidden',
      boxShadow: 'var(--inset-well)',
      ...style
    }
  }, !spectral && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: 0,
      right: 0,
      top: '50%',
      height: 1,
      background: 'var(--dd-line-1)'
    }
  }), showDisplayToggle && !missing && /*#__PURE__*/React.createElement(ViewToggle, {
    value: display,
    onChange: setDisplay
  }), missing ? /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-body)',
      color: 'var(--dd-error)'
    }
  }, "Sample file not found \u2014 region cannot be drawn") : spectral ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(Spectrogram, {
    data: data,
    kind: kind,
    reversed: reversed,
    regionStart: regionStart,
    regionEnd: regionEnd
  }), /*#__PURE__*/React.createElement("svg", {
    viewBox: `0 0 ${W} ${H}`,
    preserveAspectRatio: "none",
    style: {
      position: 'absolute',
      left: 0,
      top: 18,
      width: '100%',
      height: 'calc(100% - 18px)'
    },
    "aria-hidden": "true"
  }, fadeIn > 0 && /*#__PURE__*/React.createElement("path", {
    d: `M${(regionStart + fadeIn * regionW) * W} 0L${regionStart * W} ${H / 2}L${(regionStart + fadeIn * regionW) * W} ${H}`,
    style: {
      stroke: 'var(--dd-paper-1)',
      strokeWidth: 1.5,
      fill: 'none',
      vectorEffect: 'non-scaling-stroke',
      opacity: 0.8
    }
  }), fadeOut > 0 && /*#__PURE__*/React.createElement("path", {
    d: `M${(regionEnd - fadeOut * regionW) * W} 0L${regionEnd * W} ${H / 2}L${(regionEnd - fadeOut * regionW) * W} ${H}`,
    style: {
      stroke: 'var(--dd-paper-1)',
      strokeWidth: 1.5,
      fill: 'none',
      vectorEffect: 'non-scaling-stroke',
      opacity: 0.8
    }
  })), !compact && height >= 120 && ['20k', '2k', '200', '20'].map((f, i) => /*#__PURE__*/React.createElement("div", {
    key: f,
    style: {
      position: 'absolute',
      left: 4,
      top: `calc(18px + ${i} * (100% - 18px) / 3.4)`,
      fontFamily: 'var(--font-value)',
      fontSize: 10,
      color: 'var(--text-tertiary)',
      pointerEvents: 'none'
    }
  }, f))) : /*#__PURE__*/React.createElement("svg", {
    viewBox: `0 0 ${W} ${H}`,
    preserveAspectRatio: "none",
    style: {
      position: 'absolute',
      left: 0,
      top: 18,
      width: '100%',
      height: `calc(100% - 18px)`
    },
    "aria-hidden": "true"
  }, /*#__PURE__*/React.createElement("defs", null, /*#__PURE__*/React.createElement("clipPath", {
    id: `rg-${kind}`
  }, /*#__PURE__*/React.createElement("rect", {
    x: regionStart * W,
    y: "0",
    width: regionW * W,
    height: H
  }))), /*#__PURE__*/React.createElement("path", {
    d: path,
    style: {
      fill: 'var(--color-waveform)',
      opacity: 0.28
    }
  }), /*#__PURE__*/React.createElement("path", {
    d: path,
    clipPath: `url(#rg-${kind})`,
    style: {
      fill: 'var(--color-waveform-region)'
    }
  }), fadeIn > 0 && /*#__PURE__*/React.createElement("path", {
    d: `M${(regionStart + fadeIn * regionW) * W} 0L${regionStart * W} ${H / 2}L${(regionStart + fadeIn * regionW) * W} ${H}`,
    style: {
      stroke: 'var(--dd-paper-1)',
      strokeWidth: 1.5,
      fill: 'none',
      vectorEffect: 'non-scaling-stroke'
    }
  }), fadeIn > 0 && /*#__PURE__*/React.createElement("path", {
    d: `M${regionStart * W} 0L${(regionStart + fadeIn * regionW) * W} 0L${regionStart * W} ${H / 2}Z M${regionStart * W} ${H}L${(regionStart + fadeIn * regionW) * W} ${H}L${regionStart * W} ${H / 2}Z`,
    style: {
      fill: 'var(--surface-well)',
      opacity: 0.55
    }
  }), fadeOut > 0 && /*#__PURE__*/React.createElement("path", {
    d: `M${(regionEnd - fadeOut * regionW) * W} 0L${regionEnd * W} ${H / 2}L${(regionEnd - fadeOut * regionW) * W} ${H}`,
    style: {
      stroke: 'var(--dd-paper-1)',
      strokeWidth: 1.5,
      fill: 'none',
      vectorEffect: 'non-scaling-stroke'
    }
  }), fadeOut > 0 && /*#__PURE__*/React.createElement("path", {
    d: `M${(regionEnd - fadeOut * regionW) * W} 0L${regionEnd * W} 0L${regionEnd * W} ${H / 2}Z M${(regionEnd - fadeOut * regionW) * W} ${H}L${regionEnd * W} ${H}L${regionEnd * W} ${H / 2}Z`,
    style: {
      fill: 'var(--surface-well)',
      opacity: 0.55
    }
  })), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 0,
      bottom: 0,
      left: 0,
      width: pct(regionStart),
      background: 'var(--dd-scrim)',
      opacity: spectral ? 0.5 : 1
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 0,
      bottom: 0,
      right: 0,
      width: pct(1 - regionEnd),
      background: 'var(--dd-scrim)',
      opacity: spectral ? 0.5 : 1
    }
  }), [regionStart, regionEnd].map((p, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      position: 'absolute',
      top: 0,
      bottom: 0,
      left: pct(p),
      width: 1,
      marginLeft: i ? -1 : 0,
      background: 'var(--dd-paper-2)'
    }
  }, !slices && /*#__PURE__*/React.createElement("div", {
    style: {
      ...flag,
      left: i ? undefined : 0,
      right: i ? 0 : undefined,
      borderRadius: i ? '2px 0 0 2px' : '0 2px 2px 0',
      background: 'var(--dd-ink-5)',
      color: 'var(--text-secondary)'
    }
  }, i ? 'END' : 'START'))), loopStart != null && loopEnd != null && /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      bottom: 0,
      height: 6,
      left: pct(loopStart),
      width: pct(loopEnd - loopStart),
      background: 'var(--dd-paper-3)',
      opacity: 0.5
    }
  }), crossfade > 0 && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      bottom: 0,
      height: 6,
      left: pct(loopEnd - crossfade),
      width: pct(crossfade),
      background: 'repeating-linear-gradient(135deg, var(--dd-paper-1) 0 2px, transparent 2px 5px)'
    }
  }), [loopStart, loopEnd].map((p, i) => /*#__PURE__*/React.createElement("div", {
    key: i,
    style: {
      position: 'absolute',
      top: 18,
      bottom: 0,
      left: pct(p),
      width: 0,
      borderLeft: '1px dashed var(--dd-paper-2)'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      ...flag,
      top: 'auto',
      bottom: 8,
      left: i ? undefined : 0,
      right: i ? 0 : undefined,
      background: 'var(--dd-ink-5)',
      color: 'var(--text-primary)',
      borderRadius: 2
    }
  }, i ? 'L▸' : '◂L')))), sliceList && sliceList.map((sl, i) => {
    const p = sl.pos;
    const next = sliceList[i + 1] ? sliceList[i + 1].pos : regionEnd;
    const sel = i === selectedSlice;
    return /*#__PURE__*/React.createElement(React.Fragment, {
      key: i
    }, sel && /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        top: 18,
        bottom: 0,
        left: pct(p),
        width: pct(next - p),
        background: 'var(--dd-vermilion)',
        opacity: 0.14
      }
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'absolute',
        top: 0,
        bottom: 0,
        left: pct(p),
        width: sel ? 2 : 1,
        background: sel ? 'var(--dd-vermilion)' : 'var(--dd-paper-3)'
      }
    }), /*#__PURE__*/React.createElement("div", {
      title: sl.name ? `Slice ${i + 1} · ${sl.name}` : `Slice ${i + 1}`,
      style: {
        ...flag,
        left: pct(p),
        maxWidth: sel ? 'none' : `calc(${pct(next - p)} - 2px)`,
        minWidth: 14,
        gap: 4,
        overflow: 'hidden',
        zIndex: sel ? 2 : 1,
        background: sel ? 'var(--dd-vermilion)' : 'var(--dd-ink-5)',
        color: sel ? 'var(--text-on-accent)' : 'var(--text-secondary)',
        fontWeight: 600
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        flex: 'none'
      }
    }, i + 1), sl.name && !compact && /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-ui)',
        fontSize: 11,
        fontWeight: sel ? 700 : 600,
        overflow: 'hidden',
        textOverflow: 'ellipsis',
        color: sel ? 'var(--text-on-accent)' : 'var(--text-primary)'
      }
    }, sl.name)));
  }), startOffset > 0 && !slices && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 18,
      bottom: 0,
      left: pct(startPos),
      width: 0,
      borderLeft: `2px solid ${hostAutomated ? 'var(--dd-host)' : 'var(--dd-paper-1)'}`
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: -1,
      left: -6,
      width: 0,
      height: 0,
      borderLeft: '5px solid transparent',
      borderRight: '5px solid transparent',
      borderTop: `6px solid ${hostAutomated ? 'var(--dd-host)' : 'var(--dd-paper-1)'}`
    }
  })), startModulation && !slices && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 20,
      height: 2,
      left: pct(startPos),
      width: pct(startModulation.depth * regionW),
      background: startModulation.color || 'var(--dd-mod-a)',
      opacity: 0.8
    }
  }), cursor != null && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 18,
      bottom: 0,
      left: pct(cursor),
      width: 1,
      background: 'var(--color-cursor)',
      boxShadow: '0 0 0 1px rgba(19,15,12,0.6)'
    }
  }), reversed && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: 6,
      bottom: 6,
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      color: 'var(--text-secondary)',
      background: 'var(--dd-ink-3)',
      padding: '2px 5px',
      borderRadius: 2
    }
  }, "\u25C2 REVERSED"), label && /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      right: 6,
      top: 22,
      fontFamily: 'var(--font-value)',
      fontSize: 10,
      color: 'var(--text-tertiary)'
    }
  }, label));
}
Object.assign(__ds_scope, { makePeaks, WaveformPanel });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/WaveformPanel.jsx", error: String((e && e.message) || e) }); }

// components/icons/Icon.jsx
try { (() => {
const ICON_PATHS = {
  "reload": "<path d=\"M19 12a7 7 0 1 1-2.05-4.95\"/><path d=\"M19 4v4.5h-4.5\"/>",
  "warning": "<path d=\"M12 4 21 19.5H3Z\"/><path d=\"M12 10v4.5\"/><path d=\"M12 17.2v.3\"/>",
  "error": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"m9 9 6 6M15 9l-6 6\"/>",
  "ok": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"m8.5 12.2 2.4 2.4 4.6-4.8\"/>",
  "info": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M12 11v5\"/><path d=\"M12 8v.3\"/>",
  "lock": "<rect x=\"5.5\" y=\"10.5\" width=\"13\" height=\"9\" rx=\"1.5\"/><path d=\"M8.5 10.5V8a3.5 3.5 0 0 1 7 0v2.5\"/>",
  "chevron-down": "<path d=\"m7 10 5 5 5-5\"/>",
  "chevron-right": "<path d=\"m10 7 5 5-5 5\"/>",
  "chevron-left": "<path d=\"m14 7-5 5 5 5\"/>",
  "close": "<path d=\"m7 7 10 10M17 7 7 17\"/>",
  "plus": "<path d=\"M12 6v12M6 12h12\"/>",
  "minus": "<path d=\"M6 12h12\"/>",
  "more": "<path d=\"M6 12h.01M12 12h.01M18 12h.01\" stroke-width=\"2.5\"/>",
  "modulate": "<path d=\"M3 12c2.2-5 4.4-5 6.6 0s4.4 5 6.6 0c1.2-2.7 2.4-3.9 3.8-3.6\"/>",
  "host": "<path d=\"M9 4v5M15 4v5\"/><path d=\"M6.5 9h11v3a5.5 5.5 0 0 1-11 0Z\"/><path d=\"M12 17.5V21\"/>",
  "choke": "<path d=\"M4 8h6l2 4 2-4h6\"/><path d=\"M4 16h16\"/>",
  "alternate": "<path d=\"M5 9h11.5\"/><path d=\"m14 6 3 3-3 3\"/><path d=\"M19 15H7.5\"/><path d=\"m10 12-3 3 3 3\"/>",
  "layers": "<path d=\"M5 16h14M7 12h10M9 8h6\"/>",
  "reverse": "<path d=\"M18 7v10\"/><path d=\"M15 12H5\"/><path d=\"m8.5 8.5-3.5 3.5 3.5 3.5\"/>",
  "loop": "<path d=\"M7 9h9a3 3 0 0 1 0 6h-1.5\"/><path d=\"M16.5 15H8a3 3 0 0 1-3-3\"/><path d=\"m10 6-3 3 3 3\"/>",
  "one-shot": "<path d=\"M4 18V6\"/><path d=\"M4 7c5 0 7 3 9 7 1.2 2.4 3 4 7 4\"/>",
  "gate": "<path d=\"M3 18h3V7h12v11h3\"/>",
  "slice": "<path d=\"M6 4v16M12 4v16M18 4v16\" stroke-dasharray=\"2 2.5\"/>",
  "keyboard": "<rect x=\"3.5\" y=\"6\" width=\"17\" height=\"12\" rx=\"1.5\"/><path d=\"M8 6v7M12 6v7M16 6v7\"/>",
  "tap": "<path d=\"M10 11V5.5a1.5 1.5 0 0 1 3 0V11\"/><path d=\"M13 10.5a1.5 1.5 0 0 1 3 0V12\"/><path d=\"M16 11.5a1.5 1.5 0 0 1 3 0V15a5.5 5.5 0 0 1-5.5 5.5h-1a5 5 0 0 1-4.1-2.1L6 15.5a1.5 1.5 0 0 1 2.3-1.9L10 15\"/>",
  "pc-keys": "<rect x=\"3.5\" y=\"7\" width=\"17\" height=\"10\" rx=\"1.5\"/><path d=\"M7 10.5h.01M10 10.5h.01M13 10.5h.01M16 10.5h.01M8.5 14h7\"/>",
  "rows-more": "<path d=\"M5 6h14M5 10h14M5 14h14M5 18h14\"/>",
  "rows-less": "<path d=\"M5 9h14M5 15h14\"/>",
  "search": "<circle cx=\"10.5\" cy=\"10.5\" r=\"5.5\"/><path d=\"m15 15 4.5 4.5\"/>",
  "star": "<path d=\"m12 4.5 2.3 4.7 5.2.8-3.75 3.6.9 5.1L12 16.3l-4.65 2.4.9-5.1L4.5 10l5.2-.8Z\"/>",
  "import": "<path d=\"M12 4v10\"/><path d=\"m8 10 4 4 4-4\"/><path d=\"M5 16v2.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V16\"/>",
  "export": "<path d=\"M12 14V4\"/><path d=\"m8 8 4-4 4 4\"/><path d=\"M5 16v2.5A1.5 1.5 0 0 0 6.5 20h11a1.5 1.5 0 0 0 1.5-1.5V16\"/>",
  "folder": "<path d=\"M3.5 7.5a1.5 1.5 0 0 1 1.5-1.5h4l2 2h8a1.5 1.5 0 0 1 1.5 1.5V17a1.5 1.5 0 0 1-1.5 1.5H5A1.5 1.5 0 0 1 3.5 17Z\"/>",
  "file-missing": "<path d=\"M14 3.5H7A1.5 1.5 0 0 0 5.5 5v14A1.5 1.5 0 0 0 7 20.5h10a1.5 1.5 0 0 0 1.5-1.5V8Z\"/><path d=\"M14 3.5V8h4.5\"/><path d=\"m10 12 4 4M14 12l-4 4\"/>",
  "reset": "<path d=\"M5 12a7 7 0 1 0 2.05-4.95\"/><path d=\"M5 4v4.5h4.5\"/>",
  "midi": "<circle cx=\"12\" cy=\"12\" r=\"8\"/><path d=\"M8.5 12h.01M15.5 12h.01M10 9h.01M14 9h.01M12 8.2h.01\" stroke-width=\"2\"/>",
  "pan": "<path d=\"M3 12h18\"/><path d=\"m6 9-3 3 3 3M18 9l3 3-3 3\"/><path d=\"M12 8v8\"/>",
  "pitch": "<path d=\"M12 20V5\"/><path d=\"m8 9 4-4 4 4\"/><path d=\"M8 20h8\"/>",
  "level": "<path d=\"M5 19 19 5v14Z\"/>",
  "variation": "<rect x=\"4\" y=\"4\" width=\"16\" height=\"16\" rx=\"2\"/><path d=\"M9 9h.01M15 15h.01M15 9h.01M9 15h.01\" stroke-width=\"2.5\"/>",
  "settings": "<path d=\"M5 7h9M18 7h1M5 17h3M12 17h7\"/><circle cx=\"16\" cy=\"7\" r=\"2\"/><circle cx=\"10\" cy=\"17\" r=\"2\"/>"
};
const ICON_NAMES = Object.keys(ICON_PATHS);
function Icon({
  name,
  size = 16,
  color = 'currentColor',
  strokeWidth = 1.5,
  style,
  title
}) {
  const inner = ICON_PATHS[name];
  if (!inner) return null;
  return /*#__PURE__*/React.createElement("svg", {
    width: size,
    height: size,
    viewBox: "0 0 24 24",
    fill: "none",
    stroke: color,
    strokeWidth: strokeWidth,
    strokeLinecap: "round",
    strokeLinejoin: "round",
    style: {
      display: 'block',
      flex: 'none',
      ...style
    },
    role: title ? 'img' : undefined,
    "aria-hidden": title ? undefined : true,
    "aria-label": title,
    dangerouslySetInnerHTML: {
      __html: inner
    }
  });
}
Object.assign(__ds_scope, { ICON_NAMES, Icon });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/icons/Icon.jsx", error: String((e && e.message) || e) }); }

// components/controls/Button.jsx
try { (() => {
const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/** Text button. Variants: primary (vermilion, one per view), secondary (graphite), ghost (text only). */
function Button({
  children,
  variant = 'secondary',
  size = 'md',
  icon,
  selected = false,
  disabled = false,
  focused = false,
  pressed: pressedProp,
  onClick,
  title,
  style
}) {
  const [hover, setHover] = React.useState(false);
  const [down, setDown] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  const pressed = pressedProp ?? down;
  const h = size === 'sm' ? 24 : 28;
  let bg, fg, border;
  if (variant === 'primary') {
    bg = disabled ? 'var(--dd-ink-4)' : pressed ? 'var(--dd-vermilion-lo)' : hover ? 'var(--dd-vermilion-hi)' : 'var(--dd-vermilion)';
    fg = disabled ? 'var(--text-disabled)' : 'var(--text-on-accent)';
    border = 'transparent';
  } else if (variant === 'ghost') {
    bg = pressed ? 'var(--dd-ink-5)' : hover && !disabled ? 'var(--dd-ink-4)' : 'transparent';
    fg = disabled ? 'var(--text-disabled)' : selected ? 'var(--dd-vermilion)' : 'var(--text-secondary)';
    border = 'transparent';
  } else {
    bg = disabled ? 'var(--dd-ink-3)' : pressed ? 'var(--dd-ink-6)' : hover ? 'var(--dd-ink-5)' : 'var(--dd-ink-4)';
    fg = disabled ? 'var(--text-disabled)' : 'var(--text-primary)';
    border = selected ? 'var(--dd-vermilion)' : 'var(--border-control)';
  }
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    disabled: disabled,
    title: title,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setDown(false);
    },
    onMouseDown: () => setDown(true),
    onMouseUp: () => setDown(false),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      height: h,
      padding: size === 'sm' ? '0 8px' : '0 12px',
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 6,
      background: bg,
      color: fg,
      border: `1px solid ${border}`,
      borderRadius: 'var(--radius-2)',
      boxSizing: 'border-box',
      fontFamily: 'var(--font-ui)',
      fontSize: size === 'sm' ? 'var(--type-label)' : 'var(--type-body)',
      fontWeight: 600,
      letterSpacing: '0.02em',
      whiteSpace: 'nowrap',
      cursor: disabled ? 'default' : 'pointer',
      outline: 'none',
      boxShadow: focus || focused ? focusRing : variant === 'secondary' && !disabled && !pressed ? 'var(--highlight-cap)' : 'none',
      transform: pressed && !disabled ? 'translateY(1px)' : 'none',
      ...style
    }
  }, icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: size === 'sm' ? 14 : 16
  }), children);
}

/** Square icon-only button. Always pass `label` (tooltip + accessible name). */
function IconButton({
  icon,
  label,
  size = 'md',
  variant = 'secondary',
  selected = false,
  disabled = false,
  focused = false,
  onClick,
  style
}) {
  const [hover, setHover] = React.useState(false);
  const [down, setDown] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  const s = size === 'sm' ? 24 : 28;
  const ghost = variant === 'ghost';
  const bg = disabled ? ghost ? 'transparent' : 'var(--dd-ink-3)' : down ? 'var(--dd-ink-6)' : hover ? 'var(--dd-ink-5)' : ghost ? 'transparent' : 'var(--dd-ink-4)';
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": label,
    title: label,
    disabled: disabled,
    "aria-pressed": selected || undefined,
    onClick: onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => {
      setHover(false);
      setDown(false);
    },
    onMouseDown: () => setDown(true),
    onMouseUp: () => setDown(false),
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      width: s,
      height: s,
      padding: 0,
      display: 'inline-flex',
      alignItems: 'center',
      justifyContent: 'center',
      background: selected ? 'var(--dd-vermilion-wash)' : bg,
      borderRadius: 'var(--radius-2)',
      boxSizing: 'border-box',
      border: `1px solid ${selected ? 'var(--dd-vermilion)' : ghost ? 'transparent' : 'var(--border-control)'}`,
      color: disabled ? 'var(--text-disabled)' : selected ? 'var(--dd-vermilion)' : 'var(--text-primary)',
      cursor: disabled ? 'default' : 'pointer',
      outline: 'none',
      boxShadow: focus || focused ? focusRing : 'none',
      transform: down && !disabled ? 'translateY(1px)' : 'none',
      ...style
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: size === 'sm' ? 14 : 16
  }));
}
Object.assign(__ds_scope, { Button, IconButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/controls/Button.jsx", error: String((e && e.message) || e) }); }

// components/display/ListRow.jsx
try { (() => {
/**
 * List row (sample regions, layers, alternates, presets). 26px (22 compact).
 * Selected = vermilion 2px edge bar + raised face. `active` shows a filled dot (playing now).
 */
function ListRow({
  label,
  detail,
  value,
  icon,
  leading,
  selected = false,
  active = false,
  disabled = false,
  compact = false,
  onClick,
  trailing
}) {
  const [hover, setHover] = React.useState(false);
  return /*#__PURE__*/React.createElement("div", {
    role: "option",
    "aria-selected": selected,
    tabIndex: disabled ? -1 : 0,
    onClick: disabled ? undefined : onClick,
    onMouseEnter: () => setHover(true),
    onMouseLeave: () => setHover(false),
    onKeyDown: e => {
      if ((e.key === 'Enter' || e.key === ' ') && onClick) {
        e.preventDefault();
        onClick();
      }
    },
    style: {
      position: 'relative',
      height: compact ? 22 : 26,
      display: 'flex',
      alignItems: 'center',
      gap: 8,
      padding: '0 8px 0 10px',
      boxSizing: 'border-box',
      background: selected ? 'var(--dd-ink-4)' : hover && !disabled ? 'var(--dd-ink-3)' : 'transparent',
      borderRadius: 'var(--radius-1)',
      cursor: disabled || !onClick ? 'default' : 'pointer',
      outline: 'none',
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-label)' : 'var(--type-body)',
      fontWeight: 500,
      color: disabled ? 'var(--text-disabled)' : 'var(--text-primary)'
    },
    onFocus: e => {
      e.currentTarget.style.boxShadow = 'inset 0 0 0 2px var(--color-focus)';
    },
    onBlur: e => {
      e.currentTarget.style.boxShadow = 'none';
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      position: 'absolute',
      left: 0,
      top: 4,
      bottom: 4,
      width: 2,
      borderRadius: 1,
      background: selected ? 'var(--dd-vermilion)' : 'transparent'
    }
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      width: 6,
      height: 6,
      borderRadius: 6,
      flex: 'none',
      background: active ? 'var(--dd-paper-1)' : 'transparent',
      boxShadow: active ? 'none' : 'inset 0 0 0 1px var(--dd-line-3)'
    }
  }), leading, icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: compact ? 14 : 16,
    color: "var(--text-secondary)"
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, label), detail && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-tertiary)',
      whiteSpace: 'nowrap'
    }
  }, detail), value != null && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-micro)',
      color: 'var(--text-secondary)',
      whiteSpace: 'nowrap'
    }
  }, value), trailing);
}

/**
 * Label/value pair for settings. `prepared` = read-only patch data (lock glyph, dashed underline,
 * secondary text, no hover); live values are primary text in mono.
 */
function PropertyRow({
  label,
  value,
  unit,
  prepared = false,
  compact = false,
  hint
}) {
  return /*#__PURE__*/React.createElement("div", {
    title: prepared ? 'Prepared setting — change in the patch and reload' : hint,
    style: {
      display: 'flex',
      alignItems: 'baseline',
      gap: 8,
      minHeight: compact ? 20 : 22,
      padding: '2px 0',
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-label)' : 'var(--type-body)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-tertiary)',
      whiteSpace: 'nowrap',
      flex: 'none',
      minWidth: compact ? 64 : 92
    }
  }, label), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0,
      fontFamily: 'var(--font-value)',
      fontSize: compact ? 'var(--type-micro)' : 'var(--type-value)',
      color: prepared ? 'var(--text-secondary)' : 'var(--text-primary)',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      borderBottom: prepared ? '1px dotted var(--dd-line-3)' : 'none'
    }
  }, value), unit && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-tertiary)',
      marginLeft: 3
    }
  }, unit)), prepared && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "lock",
    size: 12,
    color: "var(--text-tertiary)",
    style: {
      alignSelf: 'center'
    }
  }));
}

/** Stacked parameter label + value for read-outs that are not themselves controls. */
function ParamLabel({
  label,
  value,
  unit,
  align = 'left',
  hostAutomated = false,
  size = 'md'
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      alignItems: align === 'center' ? 'center' : align === 'right' ? 'flex-end' : 'flex-start',
      gap: 2
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: size === 'sm' ? 'var(--type-micro)' : 'var(--type-label)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      textTransform: 'uppercase',
      color: 'var(--text-secondary)'
    }
  }, label), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: size === 'lg' ? 'var(--type-value-lg)' : size === 'sm' ? 'var(--type-micro)' : 'var(--type-value)',
      color: hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)'
    }
  }, value, unit && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-tertiary)',
      marginLeft: 3
    }
  }, unit)));
}
Object.assign(__ds_scope, { ListRow, PropertyRow, ParamLabel });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/ListRow.jsx", error: String((e && e.message) || e) }); }

// components/display/Meter.jsx
try { (() => {
const dbToPos = db => Math.max(0, Math.min(1, (db + 60) / 63)); // −60 … +3 dBFS

/** Peak meter (vertical or horizontal; mono or stereo). Zones: ok < −12 dB, warn −12…0, clip > 0 with latching LED. */
function Meter({
  levels = [-18, -20],
  peak,
  clip = false,
  orientation = 'vertical',
  length = 96,
  thickness = 6,
  compact = false,
  label,
  onResetClip
}) {
  const vertical = orientation === 'vertical';
  const zones = [{
    to: dbToPos(-12),
    c: 'var(--dd-ok)'
  }, {
    to: dbToPos(0),
    c: 'var(--dd-warn)'
  }, {
    to: 1,
    c: 'var(--dd-error)'
  }];
  const bar = (db, i) => {
    const p = dbToPos(db);
    const segs = [];
    let from = 0;
    for (const z of zones) {
      const a = from,
        b = Math.min(z.to, p);
      if (b > a) segs.push({
        a,
        b,
        c: z.c
      });
      from = z.to;
    }
    return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        position: 'relative',
        width: vertical ? thickness : length,
        height: vertical ? length : thickness,
        background: 'var(--dd-ink-0)',
        borderRadius: 1,
        overflow: 'hidden'
      }
    }, segs.map((s, j) => /*#__PURE__*/React.createElement("div", {
      key: j,
      style: vertical ? {
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: `${s.a * 100}%`,
        height: `${(s.b - s.a) * 100}%`,
        background: s.c
      } : {
        position: 'absolute',
        top: 0,
        bottom: 0,
        left: `${s.a * 100}%`,
        width: `${(s.b - s.a) * 100}%`,
        background: s.c
      }
    })), /*#__PURE__*/React.createElement("div", {
      style: vertical ? {
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: `${dbToPos(0) * 100}%`,
        height: 1,
        background: 'var(--dd-ink-2)'
      } : {
        position: 'absolute',
        top: 0,
        bottom: 0,
        left: `${dbToPos(0) * 100}%`,
        width: 1,
        background: 'var(--dd-ink-2)'
      }
    }), peak != null && /*#__PURE__*/React.createElement("div", {
      style: vertical ? {
        position: 'absolute',
        left: 0,
        right: 0,
        bottom: `${dbToPos(Array.isArray(peak) ? peak[i] : peak) * 100}%`,
        height: 2,
        background: 'var(--dd-paper-1)'
      } : {
        position: 'absolute',
        top: 0,
        bottom: 0,
        left: `${dbToPos(Array.isArray(peak) ? peak[i] : peak) * 100}%`,
        width: 2,
        background: 'var(--dd-paper-1)'
      }
    }));
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'inline-flex',
      flexDirection: vertical ? 'column' : 'row',
      alignItems: 'center',
      gap: 4
    }
  }, vertical && /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onResetClip,
    title: clip ? 'Clipped — click to reset' : 'No clipping',
    "aria-label": "Clip indicator",
    style: {
      width: thickness * levels.length + 2 * (levels.length - 1),
      height: 4,
      padding: 0,
      border: 0,
      borderRadius: 1,
      background: clip ? 'var(--dd-error)' : 'var(--dd-ink-0)',
      cursor: 'pointer'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: vertical ? 'row' : 'column',
      gap: 2
    }
  }, levels.map(bar)), label && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-caps)',
      color: 'var(--text-tertiary)'
    }
  }, label));
}
Object.assign(__ds_scope, { Meter });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/Meter.jsx", error: String((e && e.message) || e) }); }

// components/display/PadCell.jsx
try { (() => {
const NOTE_NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
/** MIDI note → name using the C4 = 60 convention (36 = C2). */
function noteName(n) {
  return NOTE_NAMES[n % 12] + (Math.floor(n / 12) - 1);
}

/**
 * Playable pad. Top: MIDI note + name. Middle: sound name. Bottom: velocity bar + alternate dots.
 * `level` (0–1) is the live activity brightness the editor decays each frame after a hit.
 * Click position sets velocity (top of pad = 127, bottom = 1).
 */
function PadCell({
  note = 36,
  name,
  mapped = true,
  selected = false,
  level = 0,
  velocity = 0,
  alternates = 0,
  activeAlternate = -1,
  layers = 0,
  activeLayer = -1,
  chokeGroup,
  choked = false,
  missing = false,
  focused = false,
  disabled = false,
  compact = false,
  size,
  onTrigger,
  onSelect,
  onRelease,
  style
}) {
  const [hover, setHover] = React.useState(false);
  const [focus, setFocus] = React.useState(false);
  const s = size ?? (compact ? 52 : 76);
  const isFocused = focused || focus;
  const lit = Math.max(0, Math.min(1, level));
  const trigger = e => {
    if (disabled || e.button !== 0) return;
    const r = e.currentTarget.getBoundingClientRect();
    const v = Math.max(0.05, Math.min(1, 1 - (e.clientY - r.top) / r.height + 0.15));
    onSelect && onSelect(note);
    if (mapped && onTrigger) onTrigger(note, v);
  };
  if (!mapped) {
    return /*#__PURE__*/React.createElement("button", {
      type: "button",
      "aria-label": `Pad ${note} ${noteName(note)}, empty`,
      onClick: () => onSelect && onSelect(note),
      onFocus: () => setFocus(true),
      onBlur: () => setFocus(false),
      onMouseEnter: () => setHover(true),
      onMouseLeave: () => setHover(false),
      style: {
        width: s,
        height: s,
        padding: compact ? 5 : 7,
        boxSizing: 'border-box',
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'space-between',
        background: hover ? 'var(--dd-ink-3)' : 'transparent',
        border: `1px dashed ${selected ? 'var(--dd-vermilion)' : 'var(--dd-line-2)'}`,
        borderRadius: 'var(--radius-2)',
        outline: 'none',
        cursor: 'pointer',
        boxShadow: isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : 'none',
        ...style
      }
    }, /*#__PURE__*/React.createElement(PadHeader, {
      note: note,
      compact: compact,
      dim: true
    }), !compact && /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-micro)',
        color: 'var(--text-disabled)',
        textAlign: 'left'
      }
    }, "Empty"));
  }
  return /*#__PURE__*/React.createElement("div", {
    role: "button",
    tabIndex: disabled ? -1 : 0,
    "aria-label": `Pad ${note} ${noteName(note)}, ${name}`,
    "aria-pressed": selected,
    onPointerDown: trigger,
    onPointerUp: () => onRelease && onRelease(note),
    onPointerLeave: () => {
      setHover(false);
    },
    onMouseEnter: () => setHover(true),
    onKeyDown: e => {
      if (e.key === ' ' || e.key === 'Enter') {
        e.preventDefault();
        onSelect && onSelect(note);
        onTrigger && onTrigger(note, 0.8);
      }
    },
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      width: s,
      height: s,
      padding: compact ? 5 : 7,
      boxSizing: 'border-box',
      position: 'relative',
      overflow: 'hidden',
      display: 'flex',
      flexDirection: 'column',
      justifyContent: 'space-between',
      userSelect: 'none',
      touchAction: 'none',
      background: disabled ? 'var(--dd-ink-3)' : hover ? 'var(--dd-pad-hover)' : 'var(--dd-pad, var(--dd-ink-4))',
      borderRadius: 'var(--radius-2)',
      outline: 'none',
      cursor: disabled ? 'default' : 'pointer',
      border: `1px solid ${selected ? 'var(--dd-vermilion)' : 'var(--dd-line-2)'}`,
      boxShadow: [isFocused ? '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)' : null, selected ? 'inset 0 0 0 1px var(--dd-vermilion)' : null, 'var(--shadow-cap)', 'var(--highlight-cap)'].filter(Boolean).join(', '),
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      inset: 0,
      background: 'var(--dd-paper-1)',
      opacity: lit * 0.16,
      pointerEvents: 'none'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: 0,
      right: 0,
      top: 0,
      height: 3,
      background: 'var(--dd-paper-1)',
      opacity: lit,
      pointerEvents: 'none'
    }
  }), /*#__PURE__*/React.createElement(PadHeader, {
    note: note,
    compact: compact,
    chokeGroup: chokeGroup
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-label)' : 'var(--type-body)',
      fontWeight: 600,
      lineHeight: 1.1,
      color: missing ? 'var(--dd-error)' : choked ? 'var(--text-tertiary)' : 'var(--text-primary)',
      textAlign: 'left',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      display: 'flex',
      alignItems: 'center',
      gap: 4
    }
  }, missing && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "file-missing",
    size: 14
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      textDecoration: choked ? 'line-through' : 'none'
    }
  }, name)), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      display: 'flex',
      alignItems: 'center',
      gap: 4,
      height: 6
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      height: 3,
      borderRadius: 1,
      background: 'var(--dd-ink-0)',
      overflow: 'hidden'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      width: `${velocity * 100}%`,
      height: '100%',
      background: 'var(--dd-paper-2)',
      opacity: 0.4 + lit * 0.6
    }
  })), layers > 1 && !compact && /*#__PURE__*/React.createElement("div", {
    title: `${layers} velocity layers`,
    style: {
      display: 'flex',
      flexDirection: 'column-reverse',
      gap: 1
    }
  }, Array.from({
    length: layers
  }).map((_, i) => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      width: 6,
      height: 2,
      borderRadius: 1,
      background: i === activeLayer ? 'var(--dd-paper-1)' : 'var(--dd-ink-6)'
    }
  }))), alternates > 1 && /*#__PURE__*/React.createElement("div", {
    title: `${alternates} alternates`,
    style: {
      display: 'flex',
      gap: 2
    }
  }, Array.from({
    length: alternates
  }).map((_, i) => /*#__PURE__*/React.createElement("span", {
    key: i,
    style: {
      width: 4,
      height: 4,
      borderRadius: 4,
      background: i === activeAlternate ? 'var(--dd-paper-1)' : 'transparent',
      boxShadow: 'inset 0 0 0 1px var(--dd-paper-3)'
    }
  })))));
}
function PadHeader({
  note,
  compact,
  dim,
  chokeGroup
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'relative',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'space-between',
      gap: 4,
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-micro)',
      lineHeight: 1,
      color: dim ? 'var(--text-disabled)' : 'var(--text-tertiary)'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      gap: 5
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      color: dim ? 'var(--text-disabled)' : 'var(--text-secondary)'
    }
  }, note), !compact && /*#__PURE__*/React.createElement("span", null, noteName(note))), chokeGroup != null && /*#__PURE__*/React.createElement("span", {
    title: `Choke group ${chokeGroup}`,
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 2,
      padding: '1px 3px',
      borderRadius: 2,
      background: 'var(--dd-ink-0)',
      color: 'var(--text-secondary)',
      fontFamily: 'var(--font-ui)',
      fontWeight: 600
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "choke",
    size: 11
  }), chokeGroup));
}
Object.assign(__ds_scope, { noteName, PadCell });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/PadCell.jsx", error: String((e && e.message) || e) }); }

// components/feedback/StatusMessage.jsx
try { (() => {
const KINDS = {
  ok: {
    icon: 'ok',
    color: 'var(--dd-ok)',
    wash: 'var(--dd-ok-wash)'
  },
  warn: {
    icon: 'warning',
    color: 'var(--dd-warn)',
    wash: 'var(--dd-warn-wash)'
  },
  error: {
    icon: 'error',
    color: 'var(--dd-error)',
    wash: 'var(--dd-error-wash)'
  },
  info: {
    icon: 'info',
    color: 'var(--dd-host)',
    wash: 'var(--dd-host-wash)'
  },
  busy: {
    icon: 'reload',
    color: 'var(--dd-paper-2)',
    wash: 'var(--dd-ink-3)'
  }
};

/**
 * Status message: icon (shape) + text + optional action. `inline` = status-bar form (no box);
 * default = banner with tinted wash and a 1px border in the status colour.
 */
function StatusMessage({
  kind = 'info',
  title,
  children,
  action,
  inline = false,
  compact = false
}) {
  const k = KINDS[kind] || KINDS.info;
  if (inline) {
    return /*#__PURE__*/React.createElement("span", {
      role: "status",
      style: {
        display: 'inline-flex',
        alignItems: 'center',
        gap: 6,
        fontFamily: 'var(--font-ui)',
        fontSize: compact ? 'var(--type-label)' : 'var(--type-body)',
        color: 'var(--text-secondary)',
        whiteSpace: 'nowrap',
        minWidth: 0
      }
    }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: k.icon,
      size: 14,
      color: k.color
    }), title && /*#__PURE__*/React.createElement("span", {
      style: {
        color: 'var(--text-primary)',
        fontWeight: 600
      }
    }, title), children && /*#__PURE__*/React.createElement("span", {
      style: {
        overflow: 'hidden',
        textOverflow: 'ellipsis'
      }
    }, children), action);
  }
  return /*#__PURE__*/React.createElement("div", {
    role: kind === 'error' ? 'alert' : 'status',
    style: {
      display: 'flex',
      alignItems: 'flex-start',
      gap: 8,
      padding: compact ? '6px 8px' : '8px 10px',
      borderRadius: 'var(--radius-2)',
      background: k.wash,
      border: `1px solid ${k.color}`,
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-body)',
      lineHeight: 'var(--leading-body)'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: k.icon,
    size: 16,
    color: k.color,
    style: {
      marginTop: 1
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1,
      minWidth: 0
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      color: 'var(--text-primary)',
      fontWeight: 600
    }
  }, title), children && /*#__PURE__*/React.createElement("div", {
    style: {
      color: 'var(--text-secondary)',
      textWrap: 'pretty'
    }
  }, children)), action && /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 'none',
      alignSelf: 'center'
    }
  }, action));
}

/** Tooltip bubble. Opens after 500 ms hover or immediately on keyboard focus; shows name, value, and modulation detail. */
function Tooltip({
  title,
  value,
  children,
  placement = 'top',
  style
}) {
  return /*#__PURE__*/React.createElement("div", {
    role: "tooltip",
    style: {
      display: 'inline-flex',
      flexDirection: 'column',
      gap: 4,
      padding: '6px 8px',
      maxWidth: 240,
      boxSizing: 'border-box',
      background: 'var(--dd-ink-0)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--radius-2)',
      boxShadow: 'var(--shadow-float)',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-label)',
      color: 'var(--text-secondary)',
      lineHeight: 1.3,
      position: 'relative',
      ...style
    }
  }, (title || value) && /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      justifyContent: 'space-between',
      gap: 12
    }
  }, title && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-primary)',
      fontWeight: 600,
      fontSize: 'var(--type-body)'
    }
  }, title), value && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      color: 'var(--text-primary)',
      fontSize: 'var(--type-value)'
    }
  }, value)), children);
}

/** Empty state for panels and pads with nothing mapped. Short, factual, one optional action. */
function EmptyState({
  icon = 'info',
  title,
  children,
  action,
  compact = false
}) {
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      alignItems: 'center',
      justifyContent: 'center',
      gap: 8,
      padding: compact ? 12 : 24,
      textAlign: 'center',
      border: '1px dashed var(--border-control)',
      borderRadius: 'var(--radius-2)',
      fontFamily: 'var(--font-ui)'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: icon,
    size: compact ? 16 : 20,
    color: "var(--text-tertiary)"
  }), title && /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 'var(--type-body)',
      fontWeight: 600,
      color: 'var(--text-secondary)'
    }
  }, title), children && /*#__PURE__*/React.createElement("div", {
    style: {
      fontSize: 'var(--type-label)',
      color: 'var(--text-tertiary)',
      maxWidth: 280,
      lineHeight: 1.35,
      textWrap: 'pretty'
    }
  }, children), action);
}
Object.assign(__ds_scope, { StatusMessage, Tooltip, EmptyState });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/feedback/StatusMessage.jsx", error: String((e && e.message) || e) }); }

// components/layout/Panel.jsx
try { (() => {
function Chevron({
  open,
  size = 12
}) {
  return /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "chevron-right",
    size: size,
    color: "var(--text-tertiary)",
    style: {
      transform: open ? 'rotate(90deg)' : 'none',
      transition: 'transform 90ms ease-out'
    }
  });
}

/**
 * Functional panel. Header (28px, 24 compact) carries a chevron, caps heading, optional tag and actions.
 * Every panel is collapsible: click the header (or Enter/Space when focused). Collapsed panels shrink to
 * their header and give their space to siblings. Panels butt together with a 4px seam; no outer shadow.
 */
function Panel({
  title,
  tag,
  prepared = false,
  actions,
  children,
  compact = false,
  padding,
  style,
  bodyStyle,
  tone = 'default',
  collapsible = true,
  collapsed: collapsedProp,
  defaultCollapsed = false,
  onToggle,
  summary
}) {
  const [local, setLocal] = React.useState(defaultCollapsed);
  const collapsed = collapsedProp ?? local;
  const toggle = () => {
    if (!collapsible) return;
    const n = !collapsed;
    if (collapsedProp == null) setLocal(n);
    onToggle && onToggle(n);
  };
  const [focus, setFocus] = React.useState(false);
  return /*#__PURE__*/React.createElement("section", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      minWidth: 0,
      minHeight: 0,
      borderRadius: 'var(--radius-3)',
      overflow: 'hidden',
      background: tone === 'sunken' ? 'var(--dd-ink-1)' : 'var(--surface-panel)',
      border: '1px solid var(--border-hairline)',
      ...style,
      ...(collapsed ? {
        flex: 'none'
      } : null)
    }
  }, title && /*#__PURE__*/React.createElement("header", {
    role: collapsible ? 'button' : undefined,
    tabIndex: collapsible ? 0 : undefined,
    "aria-expanded": collapsible ? !collapsed : undefined,
    onClick: e => {
      if (e.target.closest('[data-panel-action]')) return;
      toggle();
    },
    onKeyDown: e => {
      if (e.target !== e.currentTarget) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggle();
      }
    },
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    style: {
      height: compact ? 24 : 28,
      flex: 'none',
      display: 'flex',
      alignItems: 'center',
      gap: compact ? 6 : 8,
      padding: compact ? '0 8px 0 6px' : '0 12px 0 8px',
      background: 'var(--surface-panel-header)',
      borderBottom: collapsed ? 'none' : '1px solid var(--border-hairline)',
      cursor: collapsible ? 'pointer' : 'default',
      outline: 'none',
      userSelect: 'none',
      boxShadow: focus ? 'inset 0 0 0 2px var(--color-focus)' : 'none'
    }
  }, collapsible && /*#__PURE__*/React.createElement(Chevron, {
    open: !collapsed
  }), /*#__PURE__*/React.createElement(SectionHeading, {
    compact: compact,
    prepared: prepared
  }, title), tag && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 10,
      fontWeight: 600,
      letterSpacing: '0.08em',
      textTransform: 'uppercase',
      padding: '1px 4px',
      borderRadius: 2,
      border: '1px solid var(--border-strong)',
      color: 'var(--text-secondary)'
    }
  }, tag), collapsed && summary && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-micro)',
      color: 'var(--text-tertiary)',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis',
      minWidth: 0
    }
  }, summary), /*#__PURE__*/React.createElement("div", {
    style: {
      flex: 1
    }
  }), actions && /*#__PURE__*/React.createElement("div", {
    "data-panel-action": "",
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6
    }
  }, actions)), !collapsed && /*#__PURE__*/React.createElement("div", {
    className: "dd-scroll",
    style: {
      flex: 1,
      minHeight: 0,
      padding: padding ?? (compact ? 8 : 12),
      ...bodyStyle
    }
  }, children));
}

/**
 * Rollout — collapsible region inside a panel (3ds Max-style). 22px header: chevron, caps title,
 * hairline rule, and a one-line summary when collapsed so key values stay visible.
 */
function Rollout({
  title,
  summary,
  children,
  collapsed: collapsedProp,
  defaultCollapsed = false,
  onToggle,
  prepared = false,
  compact = false,
  actions
}) {
  const [local, setLocal] = React.useState(defaultCollapsed);
  const collapsed = collapsedProp ?? local;
  const toggle = () => {
    const n = !collapsed;
    if (collapsedProp == null) setLocal(n);
    onToggle && onToggle(n);
  };
  const [focus, setFocus] = React.useState(false);
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column'
    }
  }, /*#__PURE__*/React.createElement("div", {
    role: "button",
    tabIndex: 0,
    "aria-expanded": !collapsed,
    onClick: e => {
      if (e.target.closest('[data-panel-action]')) return;
      toggle();
    },
    onKeyDown: e => {
      if (e.target !== e.currentTarget) return;
      if (e.key === 'Enter' || e.key === ' ') {
        e.preventDefault();
        toggle();
      }
      if (e.key === 'ArrowLeft' && !collapsed) toggle();
      if (e.key === 'ArrowRight' && collapsed) toggle();
    },
    onFocus: () => setFocus(true),
    onBlur: () => setFocus(false),
    onMouseEnter: e => {
      e.currentTarget.style.background = 'var(--dd-ink-3)';
    },
    onMouseLeave: e => {
      e.currentTarget.style.background = 'transparent';
    },
    style: {
      height: compact ? 20 : 22,
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      padding: '0 4px',
      margin: '0 -4px',
      borderRadius: 'var(--radius-1)',
      cursor: 'pointer',
      outline: 'none',
      userSelect: 'none',
      boxShadow: focus ? 'inset 0 0 0 2px var(--color-focus)' : 'none'
    }
  }, /*#__PURE__*/React.createElement(Chevron, {
    open: !collapsed,
    size: 12
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 5,
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 700,
      letterSpacing: 'var(--tracking-heading)',
      textTransform: 'uppercase',
      color: 'var(--text-secondary)',
      whiteSpace: 'nowrap'
    }
  }, prepared && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "lock",
    size: 11,
    color: "var(--text-tertiary)"
  }), title), collapsed && summary ? /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      minWidth: 0,
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-micro)',
      color: 'var(--text-tertiary)',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, summary) : /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      height: 1,
      background: 'var(--border-hairline)'
    }
  }), actions && /*#__PURE__*/React.createElement("div", {
    "data-panel-action": "",
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 4
    }
  }, actions)), !collapsed && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: compact ? '2px 0 6px 18px' : '4px 0 8px 18px'
    }
  }, children));
}

/** Caps section heading. `prepared` adds a lock glyph to mark read-only patch settings. */
function SectionHeading({
  children,
  compact = false,
  prepared = false,
  color
}) {
  return /*#__PURE__*/React.createElement("h3", {
    style: {
      margin: 0,
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-label)' : 'var(--type-heading)',
      fontWeight: 700,
      letterSpacing: 'var(--tracking-heading)',
      textTransform: 'uppercase',
      color: color || 'var(--text-secondary)',
      whiteSpace: 'nowrap'
    }
  }, prepared && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "lock",
    size: 12,
    color: "var(--text-tertiary)"
  }), children);
}
Object.assign(__ds_scope, { Panel, Rollout, SectionHeading });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/layout/Panel.jsx", error: String((e && e.message) || e) }); }

// components/navigation/ContextMenu.jsx
try { (() => {
/**
 * Context menu / popup menu. Items: { label, icon, shortcut, disabled, danger, checked, submenu, header, separator, onSelect }.
 * Special item `{ type: 'assignment', slot, source, depth, onDepth, onRemove }` renders an inline depth slider row.
 * Keyboard: ↑/↓ move, Enter select, ←/→ adjust a focused depth row (Shift = fine), Delete removes it, Esc closes.
 */
function ContextMenu({
  items = [],
  title,
  x,
  y,
  width = 248,
  onClose,
  style
}) {
  const [active, setActive] = React.useState(() => items.findIndex(i => isFocusable(i)));
  const ref = React.useRef(null);
  React.useEffect(() => {
    ref.current && ref.current.focus();
  }, []);
  const move = dir => {
    let i = active;
    for (let n = 0; n < items.length; n++) {
      i = (i + dir + items.length) % items.length;
      if (isFocusable(items[i])) break;
    }
    setActive(i);
  };
  const onKeyDown = e => {
    const it = items[active];
    if (e.key === 'ArrowDown') {
      move(1);
      e.preventDefault();
    } else if (e.key === 'ArrowUp') {
      move(-1);
      e.preventDefault();
    } else if (e.key === 'Escape') {
      onClose && onClose();
    } else if (e.key === 'Enter' && it && !it.disabled && it.onSelect) {
      it.onSelect();
    } else if (it && it.type === 'assignment' && (e.key === 'ArrowLeft' || e.key === 'ArrowRight')) {
      const d = (e.key === 'ArrowRight' ? 1 : -1) * (e.shiftKey ? 0.01 : 0.05);
      it.onDepth && it.onDepth(Math.max(-1, Math.min(1, (it.depth || 0) + d)));
      e.preventDefault();
    } else if (it && it.type === 'assignment' && (e.key === 'Delete' || e.key === 'Backspace')) {
      it.onRemove && it.onRemove();
    }
  };
  return /*#__PURE__*/React.createElement("div", {
    ref: ref,
    role: "menu",
    tabIndex: -1,
    onKeyDown: onKeyDown,
    style: {
      position: x != null ? 'fixed' : 'relative',
      left: x,
      top: y,
      width,
      boxSizing: 'border-box',
      padding: '4px 0',
      outline: 'none',
      zIndex: 100,
      background: 'var(--surface-menu)',
      border: '1px solid var(--border-strong)',
      borderRadius: 'var(--radius-2)',
      boxShadow: 'var(--shadow-float)',
      fontFamily: 'var(--font-ui)',
      color: 'var(--text-primary)',
      ...style
    }
  }, title && /*#__PURE__*/React.createElement("div", {
    style: {
      padding: '4px 10px 6px',
      fontSize: 'var(--type-micro)',
      fontWeight: 600,
      letterSpacing: 'var(--tracking-heading)',
      textTransform: 'uppercase',
      color: 'var(--text-tertiary)',
      borderBottom: '1px solid var(--border-hairline)',
      marginBottom: 4
    }
  }, title), items.map((it, i) => {
    if (it.separator) return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        height: 1,
        background: 'var(--border-hairline)',
        margin: '4px 0'
      }
    });
    if (it.header) return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        padding: '6px 10px 2px',
        fontSize: 'var(--type-micro)',
        fontWeight: 600,
        letterSpacing: 'var(--tracking-heading)',
        textTransform: 'uppercase',
        color: 'var(--text-tertiary)'
      }
    }, it.header);
    if (it.note) return /*#__PURE__*/React.createElement("div", {
      key: i,
      style: {
        padding: '4px 10px 6px',
        fontSize: 'var(--type-micro)',
        lineHeight: 1.35,
        color: 'var(--text-tertiary)',
        textWrap: 'pretty'
      }
    }, it.note);
    if (it.type === 'assignment') return /*#__PURE__*/React.createElement(AssignmentRow, {
      key: i,
      it: it,
      active: i === active,
      onHover: () => setActive(i)
    });
    const isActive = i === active && !it.disabled;
    return /*#__PURE__*/React.createElement("div", {
      key: i,
      role: "menuitem",
      "aria-disabled": it.disabled || undefined,
      onMouseEnter: () => setActive(i),
      onClick: () => !it.disabled && it.onSelect && it.onSelect(),
      style: {
        height: 26,
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        padding: '0 10px',
        margin: '0 4px',
        borderRadius: 3,
        cursor: it.disabled ? 'default' : 'pointer',
        background: isActive ? 'var(--dd-ink-5)' : 'transparent',
        fontSize: 'var(--type-body)',
        fontWeight: 500,
        color: it.disabled ? 'var(--text-disabled)' : it.danger ? 'var(--dd-error)' : 'var(--text-primary)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        width: 16,
        display: 'flex',
        justifyContent: 'center',
        color: it.disabled ? 'var(--text-disabled)' : 'var(--text-secondary)'
      }
    }, it.checked ? /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: "ok",
      size: 14
    }) : it.icon ? /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: it.icon,
      size: 16
    }) : null), /*#__PURE__*/React.createElement("span", {
      style: {
        flex: 1,
        whiteSpace: 'nowrap',
        overflow: 'hidden',
        textOverflow: 'ellipsis'
      }
    }, it.label), it.shortcut && /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-value)',
        fontSize: 'var(--type-micro)',
        color: 'var(--text-tertiary)'
      }
    }, it.shortcut), it.submenu && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: "chevron-right",
      size: 14,
      color: "var(--text-tertiary)"
    }));
  }));
}
function isFocusable(i) {
  return i && !i.separator && !i.header && !i.note && !i.disabled;
}
function AssignmentRow({
  it,
  active,
  onHover
}) {
  const slot = __ds_scope.MOD_SLOTS[it.slot] || __ds_scope.MOD_SLOTS.A;
  const d = it.depth || 0;
  const trackRef = React.useRef(null);
  const setFrom = e => {
    const r = trackRef.current.getBoundingClientRect();
    const v = (e.clientX - r.left) / r.width * 2 - 1;
    it.onDepth && it.onDepth(Math.max(-1, Math.min(1, Math.round(v * 100) / 100)));
  };
  return /*#__PURE__*/React.createElement("div", {
    role: "menuitem",
    onMouseEnter: onHover,
    style: {
      display: 'grid',
      gridTemplateColumns: '16px 1fr auto 20px',
      alignItems: 'center',
      columnGap: 8,
      rowGap: 4,
      padding: '5px 6px 6px 10px',
      margin: '0 4px',
      borderRadius: 3,
      background: active ? 'var(--dd-ink-5)' : 'transparent'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      justifyContent: 'center'
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.ModGlyph, {
    slot: it.slot,
    size: 10
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      fontSize: 'var(--type-body)',
      fontWeight: 500,
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, it.source), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-value)',
      fontSize: 'var(--type-micro)',
      color: slot.color
    }
  }, d > 0 ? '+' : d < 0 ? '−' : '', Math.abs(Math.round(d * 100)), "%"), /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": `Remove ${it.source}`,
    title: "Remove assignment",
    onClick: it.onRemove,
    style: {
      width: 20,
      height: 20,
      padding: 0,
      border: 0,
      borderRadius: 2,
      background: 'transparent',
      color: 'var(--text-tertiary)',
      cursor: 'pointer',
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center'
    },
    onMouseEnter: e => {
      e.currentTarget.style.color = 'var(--dd-error)';
    },
    onMouseLeave: e => {
      e.currentTarget.style.color = 'var(--text-tertiary)';
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "close",
    size: 14
  })), /*#__PURE__*/React.createElement("span", null), /*#__PURE__*/React.createElement("div", {
    ref: trackRef,
    onPointerDown: e => {
      e.currentTarget.setPointerCapture(e.pointerId);
      setFrom(e);
    },
    onPointerMove: e => {
      if (e.buttons) setFrom(e);
    },
    onDoubleClick: () => it.onDepth && it.onDepth(0),
    style: {
      gridColumn: '2 / 4',
      position: 'relative',
      height: 12,
      cursor: 'ew-resize',
      touchAction: 'none'
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: 0,
      right: 0,
      top: 5,
      height: 2,
      background: 'var(--dd-ink-0)',
      borderRadius: 1
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      left: '50%',
      top: 2,
      width: 1,
      height: 8,
      background: 'var(--dd-paper-4)'
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 5,
      height: 2,
      background: slot.color,
      left: `${50 + Math.min(0, d) * 50}%`,
      width: `${Math.abs(d) * 50}%`
    }
  }), /*#__PURE__*/React.createElement("div", {
    style: {
      position: 'absolute',
      top: 1,
      width: 4,
      height: 10,
      marginLeft: -2,
      borderRadius: 1,
      background: 'var(--dd-paper-1)',
      left: `${50 + d * 50}%`
    }
  })), /*#__PURE__*/React.createElement("span", null));
}

/** Dropdown menu trigger (closed state). Opens a ContextMenu anchored below. */
function MenuButton({
  label,
  value,
  onClick,
  compact = false,
  disabled = false,
  width,
  readOnly = false
}) {
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    onClick: onClick,
    disabled: disabled || readOnly,
    style: {
      height: compact ? 22 : 24,
      width,
      minWidth: 96,
      padding: '0 6px 0 8px',
      display: 'inline-flex',
      alignItems: 'center',
      gap: 6,
      boxSizing: 'border-box',
      background: readOnly ? 'transparent' : 'var(--surface-well)',
      border: `1px ${readOnly ? 'dashed' : 'solid'} var(--border-control)`,
      borderRadius: 'var(--radius-1)',
      color: disabled ? 'var(--text-disabled)' : 'var(--text-primary)',
      cursor: disabled || readOnly ? 'default' : 'pointer',
      fontFamily: 'var(--font-ui)',
      fontSize: compact ? 'var(--type-label)' : 'var(--type-body)',
      fontWeight: 500
    }
  }, label && /*#__PURE__*/React.createElement("span", {
    style: {
      color: 'var(--text-tertiary)'
    }
  }, label), /*#__PURE__*/React.createElement("span", {
    style: {
      flex: 1,
      textAlign: 'left',
      whiteSpace: 'nowrap',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    }
  }, value), !readOnly && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
    name: "chevron-down",
    size: 14,
    color: "var(--text-tertiary)"
  }));
}
Object.assign(__ds_scope, { ContextMenu, MenuButton });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/navigation/ContextMenu.jsx", error: String((e && e.message) || e) }); }

// components/display/LayerStack.jsx
try { (() => {
function _extends() { return _extends = Object.assign ? Object.assign.bind() : function (n) { for (var e = 1; e < arguments.length; e++) { var t = arguments[e]; for (var r in t) ({}).hasOwnProperty.call(t, r) && (n[r] = t[r]); } return n; }, _extends.apply(null, arguments); }
const lsSendNorm = db => db == null || db <= -60 ? 0 : (db + 60) / 66;
const lsSendDb = n => n <= 0.001 ? null : Math.round((-60 + n * 66) * 10) / 10;
const lsSendText = db => db == null ? '−∞ dB' : db === 0 ? '0.0 dB' : `${db < 0 ? '−' : '+'}${Math.abs(db).toFixed(1)} dB`;
const lsLabel = {
  fontFamily: 'var(--font-ui)',
  fontSize: 'var(--type-micro)',
  fontWeight: 600,
  letterSpacing: '0.08em',
  textTransform: 'uppercase',
  color: 'var(--text-tertiary)'
};
const lsValue = {
  fontFamily: 'var(--font-value)',
  fontSize: 'var(--type-micro)',
  color: 'var(--text-secondary)',
  whiteSpace: 'nowrap'
};
const lsFmt = (p, v = p.value) => {
  if (p.options) return String(v);
  if (p.unit === 'Hz') return v >= 1000 ? `${(v / 1000).toFixed(v >= 10000 ? 0 : 1)} kHz` : `${Math.round(v)} Hz`;
  if (p.unit === 'dB') return v === 0 ? '0.0 dB' : `${v < 0 ? '−' : '+'}${Math.abs(v).toFixed(1)} dB`;
  if (p.unit === '%') return `${Math.round(v)}%`;
  if (p.unit === 'ms') return `${Math.round(v)} ms`;
  return `${(+v).toFixed(2)}${p.unit ? ' ' + p.unit : ''}`;
};
const lsNorm = (p, v) => p.log ? Math.log(v / p.min) / Math.log(p.max / p.min) : (v - p.min) / (p.max - p.min);
const lsDenorm = (p, n) => p.log ? p.min * Math.pow(p.max / p.min, n) : p.min + n * (p.max - p.min);
const lsSummary = m => {
  const s = (m.params || []).filter(p => p.summary);
  return s.length ? s.map(p => lsFmt(p)).join(' ') : m.value;
};
const lsDb = v => v === 0 ? '0.0 dB' : `${v < 0 ? '−' : '+'}${Math.abs(v).toFixed(1)} dB`;

/**
 * Layers that all trigger together for one zone or pad. Each layer is a source (synth engine, sample, patch)
 * followed by an inline chain of modules (filter, gain, saturation…) processed left to right.
 * Optional routing per row: a send knob for each FX bus (fxBusses) and an output menu (outputs). Use the same component for FX busses (rows with source "FX bus").
 * Click the source block to open the source editor (sample: waveform + playback params; synth: engine params).
 * Click a module to open its editor under the layer (click again or Esc to close); its dot bypasses, × removes; the dashed slot adds a module.
 */
function LayerStack({
  layers = [],
  selectedId,
  onSelect,
  onChange,
  onAddModule,
  fxBusses = [],
  outputs = [],
  countLabel,
  title = 'Layers',
  subtitle,
  editable = true,
  style
}) {
  const [menu, setMenu] = React.useState(null);
  const [local, setLocal] = React.useState(layers);
  const ref = React.useRef(layers);
  React.useEffect(() => {
    setLocal(layers);
    ref.current = layers;
  }, [layers]);
  const [sel, setSel] = React.useState(selectedId ?? layers[0]?.id);
  React.useEffect(() => {
    if (selectedId !== undefined) setSel(selectedId);
  }, [selectedId]);
  const [selMod, setSelMod] = React.useState(null);
  const [hoverMod, setHoverMod] = React.useState(null);
  const select = id => {
    setSel(id);
    onSelect && onSelect(id);
  };
  const commit = (next, id) => {
    ref.current = next;
    setLocal(next);
    onChange && onChange(next, id);
  };
  const patchLayer = (lid, fn) => commit(ref.current.map(l => l.id === lid ? fn(l) : l), lid);
  const toggleMute = l => patchLayer(l.id, o => ({
    ...o,
    muted: !o.muted
  }));
  const toggleBypass = (l, m) => patchLayer(l.id, o => ({
    ...o,
    modules: o.modules.map(x => x.id === m.id ? {
      ...x,
      bypassed: !x.bypassed
    } : x)
  }));
  const setParam = (l, m, pid, v) => patchLayer(l.id, o => ({
    ...o,
    modules: o.modules.map(x => x.id === m.id ? {
      ...x,
      params: x.params.map(p => p.id === pid ? {
        ...p,
        value: v
      } : p)
    } : x)
  }));
  const setSrcParam = (l, pid, v) => patchLayer(l.id, o => ({
    ...o,
    params: o.params.map(p => p.id === pid ? {
      ...p,
      value: v
    } : p)
  }));
  const removeMod = (l, m) => patchLayer(l.id, o => ({
    ...o,
    modules: o.modules.filter(x => x.id !== m.id)
  }));
  const stop = e => e.stopPropagation();
  const box = {
    boxSizing: 'border-box',
    height: 38,
    borderRadius: 'var(--radius-2, 4px)',
    display: 'flex',
    flexDirection: 'column',
    justifyContent: 'center',
    gap: 3,
    padding: '0 8px',
    flex: 'none'
  };
  const wire = /*#__PURE__*/React.createElement("span", {
    style: {
      width: 8,
      height: 1,
      background: 'var(--dd-line-3)',
      flex: 'none'
    }
  });
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 4,
      minWidth: 0,
      userSelect: 'none',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      gap: 10,
      minHeight: 18
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: { ...lsLabel, color: 'var(--dd-secondary, var(--text-secondary))' }
  }, title), subtitle && /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-label)',
      fontWeight: 600,
      color: 'var(--text-primary)',
      whiteSpace: 'nowrap'
    }
  }, subtitle), /*#__PURE__*/React.createElement("span", {
    style: {
      ...lsValue,
      color: 'var(--text-tertiary)',
      marginLeft: 'auto'
    }
  }, countLabel ?? `${local.length} ${local.length === 1 ? 'layer' : 'layers'} · all trigger`)), local.map(l => {
    const isSel = l.id === sel;
    const srcKey = l.id + '::src',
      srcOpen = selMod === srcKey;
    const mod = (l.modules || []).find(m => selMod === l.id + m.id);
    const openMod = srcOpen ? {
      id: '::src',
      type: `${l.source || 'Source'} · ${l.name}`,
      params: l.params,
      src: true
    } : mod;
    const setP = (pid, v) => srcOpen ? setSrcParam(l, pid, v) : setParam(l, mod, pid, v);
    const startP = srcOpen && (l.params || []).find(p => p.id === 'start');
    const hasSrcEditor = !!(l.params || l.waveform);
    const outName = (outputs.find(o => o.id === l.output) || outputs[0] || {}).name;
    return /*#__PURE__*/React.createElement("div", {
      key: l.id,
      style: {
        display: 'flex',
        flexDirection: 'column'
      }
    }, /*#__PURE__*/React.createElement("div", {
      role: "button",
      tabIndex: 0,
      "aria-pressed": isSel,
      onPointerDown: () => select(l.id),
      onKeyDown: e => {
        if (e.target === e.currentTarget && (e.key === ' ' || e.key === 'Enter')) {
          e.preventDefault();
          select(l.id);
        }
      },
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: 6,
        background: isSel ? 'var(--dd-ink-4)' : 'var(--dd-ink-3)',
        border: `1px solid ${isSel ? 'var(--dd-vermilion)' : 'var(--border-hairline)'}`,
        borderRadius: openMod ? '4px 4px 0 0' : 'var(--radius-2, 4px)',
        cursor: 'pointer',
        outline: 'none'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        minWidth: 0,
        display: 'flex',
        alignItems: 'center',
        flexWrap: 'wrap',
        rowGap: 4,
        opacity: l.muted ? 0.5 : 1
      }
    }, /*#__PURE__*/React.createElement("div", {
      role: "button",
      tabIndex: 0,
      "aria-expanded": srcOpen,
      "aria-label": `Edit ${l.source || 'source'} ${l.name}`,
      onPointerDown: e => {
        stop(e);
        select(l.id);
        if (hasSrcEditor) setSelMod(srcOpen ? null : srcKey);
      },
      onKeyDown: e => {
        if (e.key === ' ' || e.key === 'Enter') {
          e.preventDefault();
          stop(e);
          setSelMod(srcOpen ? null : srcKey);
        }
        if (e.key === 'Escape') setSelMod(null);
      },
      onPointerEnter: () => setHoverMod(srcKey),
      onPointerLeave: () => setHoverMod(null),
      style: {
        ...box,
        minWidth: 190,
        maxWidth: 280,
        cursor: hasSrcEditor ? 'pointer' : 'default',
        background: hoverMod === srcKey && hasSrcEditor ? 'var(--dd-ink-3)' : 'var(--dd-ink-2)',
        border: `1px solid ${srcOpen ? 'var(--dd-vermilion)' : 'var(--border-control)'}`,
        outline: 'none'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 6
      }
    }, l.source && /*#__PURE__*/React.createElement("span", {
      style: {
        ...lsLabel,
        padding: '1px 4px',
        background: 'var(--dd-ink-0)',
        borderRadius: 2,
        color: 'var(--text-secondary)',
        flex: 'none'
      }
    }, l.source), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-label)',
        fontWeight: 600,
        color: 'var(--text-primary)',
        whiteSpace: 'nowrap',
        flex: 'none'
      }
    }, l.name)), l.detail && /*#__PURE__*/React.createElement("span", {
      style: {
        ...lsValue,
        color: 'var(--text-tertiary)',
        overflow: 'hidden',
        textOverflow: 'ellipsis'
      }
    }, l.detail)), (l.modules || []).map(m => {
      const key = l.id + m.id,
        mSel = selMod === key,
        hot = hoverMod === key;
      return /*#__PURE__*/React.createElement(React.Fragment, {
        key: m.id
      }, wire, /*#__PURE__*/React.createElement("div", {
        role: "button",
        tabIndex: 0,
        "aria-label": `${m.type}${m.bypassed ? ', bypassed' : ''}`,
        onPointerDown: e => {
          stop(e);
          select(l.id);
          setSelMod(mSel ? null : key);
        },
        onKeyDown: e => {
          if (e.key === ' ' || e.key === 'Enter') {
            e.preventDefault();
            stop(e);
            setSelMod(mSel ? null : key);
          }
          if (e.key === 'Escape') setSelMod(null);
        },
        "aria-expanded": mSel,
        onPointerEnter: () => setHoverMod(key),
        onPointerLeave: () => setHoverMod(null),
        style: {
          ...box,
          position: 'relative',
          minWidth: 88,
          background: hot ? 'var(--surface-control-hover)' : 'var(--surface-control)',
          border: `1px solid ${mSel ? 'var(--dd-vermilion)' : 'var(--border-control)'}`,
          outline: 'none',
          opacity: m.bypassed ? 0.55 : 1
        }
      }, /*#__PURE__*/React.createElement("span", {
        style: {
          display: 'flex',
          alignItems: 'center',
          gap: 5
        }
      }, /*#__PURE__*/React.createElement("button", {
        type: "button",
        "aria-label": m.bypassed ? 'Enable module' : 'Bypass module',
        "aria-pressed": !m.bypassed,
        onPointerDown: stop,
        onClick: e => {
          stop(e);
          toggleBypass(l, m);
        },
        title: m.bypassed ? 'Bypassed' : 'On',
        style: {
          width: 8,
          height: 8,
          padding: 0,
          borderRadius: 8,
          cursor: 'pointer',
          background: m.bypassed ? 'transparent' : 'var(--dd-paper-1)',
          border: `1px solid ${m.bypassed ? 'var(--dd-paper-3)' : 'var(--dd-paper-1)'}`,
          flex: 'none'
        }
      }), /*#__PURE__*/React.createElement("span", {
        style: {
          ...lsLabel,
          color: 'var(--text-secondary)',
          textDecoration: m.bypassed ? 'line-through' : 'none'
        }
      }, m.type), editable && hot && /*#__PURE__*/React.createElement("button", {
        type: "button",
        "aria-label": `Remove ${m.type}`,
        onPointerDown: stop,
        onClick: e => {
          stop(e);
          removeMod(l, m);
        },
        style: {
          marginLeft: 'auto',
          width: 14,
          height: 14,
          padding: 0,
          background: 'transparent',
          border: 'none',
          color: 'var(--text-tertiary)',
          fontFamily: 'var(--font-ui)',
          fontSize: 'var(--type-label)',
          lineHeight: 1,
          cursor: 'pointer'
        }
      }, "\xD7")), /*#__PURE__*/React.createElement("span", {
        style: {
          ...lsValue,
          color: 'var(--text-primary)'
        }
      }, lsSummary(m))));
    }), editable && /*#__PURE__*/React.createElement(React.Fragment, null, wire, /*#__PURE__*/React.createElement("button", {
      type: "button",
      "aria-label": `Add module to ${l.name}`,
      title: "Add module\u2026",
      onPointerDown: stop,
      onClick: e => {
        stop(e);
        onAddModule && onAddModule(l.id);
      },
      style: {
        ...box,
        width: 30,
        padding: 0,
        alignItems: 'center',
        background: 'transparent',
        border: '1px dashed var(--border-strong)',
        color: 'var(--text-tertiary)',
        fontFamily: 'var(--font-value)',
        fontSize: 'var(--type-body)',
        cursor: 'pointer'
      }
    }, "+"))), fxBusses.length > 0 && /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 8,
        flex: 'none',
        paddingLeft: 4,
        borderLeft: '1px solid var(--border-hairline)'
      },
      onPointerDown: stop
    }, fxBusses.map(b => {
      const db = (l.sends || {})[b.id];
      return /*#__PURE__*/React.createElement("div", {
        key: b.id,
        title: `Send to ${b.name}`,
        style: {
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 2
        }
      }, /*#__PURE__*/React.createElement("span", {
        style: {
          ...lsLabel,
          color: db == null ? 'var(--text-disabled)' : 'var(--text-secondary)'
        }
      }, b.letter || b.name), /*#__PURE__*/React.createElement(__ds_scope.Knob, {
        size: "xs",
        showLabel: false,
        label: `Send ${b.name}`,
        value: lsSendNorm(db),
        defaultValue: 0,
        valueText: lsSendText(db),
        onChange: n => patchLayer(l.id, o => ({
          ...o,
          sends: {
            ...(o.sends || {}),
            [b.id]: lsSendDb(n)
          }
        }))
      }));
    })), outputs.length > 0 && /*#__PURE__*/React.createElement("span", {
      onPointerDown: stop,
      style: {
        flex: 'none'
      },
      onClickCapture: e => {
        const r = e.currentTarget.getBoundingClientRect();
        setMenu(menu && menu.id === l.id ? null : {
          id: l.id,
          x: r.left,
          y: r.bottom + 4
        });
      }
    }, /*#__PURE__*/React.createElement(__ds_scope.MenuButton, {
      compact: true,
      width: 124,
      value: outName
    })), /*#__PURE__*/React.createElement("span", {
      style: {
        ...lsValue,
        width: 56,
        textAlign: 'right',
        color: 'var(--text-primary)',
        flex: 'none'
      }
    }, lsDb(l.level ?? 0)), /*#__PURE__*/React.createElement("button", {
      type: "button",
      "aria-label": `${l.muted ? 'Unmute' : 'Mute'} ${l.name}`,
      "aria-pressed": !!l.muted,
      onPointerDown: stop,
      onClick: e => {
        stop(e);
        toggleMute(l);
      },
      style: {
        width: 22,
        height: 20,
        padding: 0,
        flex: 'none',
        background: l.muted ? 'var(--dd-warn)' : 'var(--surface-control)',
        color: l.muted ? 'var(--text-on-accent)' : 'var(--text-secondary)',
        border: '1px solid var(--border-control)',
        borderRadius: 'var(--radius-2, 4px)',
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-micro)',
        fontWeight: 700,
        cursor: 'pointer'
      }
    }, "M")), openMod && /*#__PURE__*/React.createElement("div", {
      role: "region",
      "aria-label": `${openMod.type} editor`,
      onKeyDown: e => {
        if (e.key === 'Escape') setSelMod(null);
      },
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 10,
        padding: '10px 12px 12px',
        background: 'var(--dd-ink-2)',
        border: '1px solid var(--dd-vermilion)',
        borderTop: '1px solid var(--border-hairline)',
        borderRadius: '0 0 4px 4px',
        opacity: openMod.bypassed ? 0.6 : 1
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 8
      }
    }, !openMod.src && /*#__PURE__*/React.createElement("button", {
      type: "button",
      "aria-label": openMod.bypassed ? 'Enable module' : 'Bypass module',
      "aria-pressed": !openMod.bypassed,
      onClick: () => toggleBypass(l, openMod),
      style: {
        width: 10,
        height: 10,
        padding: 0,
        borderRadius: 10,
        cursor: 'pointer',
        background: openMod.bypassed ? 'transparent' : 'var(--dd-paper-1)',
        border: `1px solid ${openMod.bypassed ? 'var(--dd-paper-3)' : 'var(--dd-paper-1)'}`
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-label)',
        fontWeight: 600,
        letterSpacing: '0.06em',
        textTransform: 'uppercase',
        color: 'var(--text-primary)'
      }
    }, openMod.src ? l.source || 'Source' : openMod.type), /*#__PURE__*/React.createElement("span", {
      style: {
        ...lsValue,
        color: 'var(--text-tertiary)'
      }
    }, openMod.src ? l.detail || l.name : l.name, openMod.bypassed ? ' · bypassed' : ''), /*#__PURE__*/React.createElement("button", {
      type: "button",
      "aria-label": "Close editor",
      onClick: () => setSelMod(null),
      style: {
        marginLeft: 'auto',
        width: 20,
        height: 20,
        padding: 0,
        background: 'var(--surface-control)',
        border: '1px solid var(--border-control)',
        borderRadius: 'var(--radius-2, 4px)',
        color: 'var(--text-secondary)',
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-label)',
        lineHeight: 1,
        cursor: 'pointer'
      }
    }, "\xD7")), openMod.src && l.waveform && /*#__PURE__*/React.createElement(__ds_scope.WaveformPanel, _extends({
      height: 96
    }, l.waveform, {
      startOffset: startP ? startP.value / 100 : l.waveform.startOffset
    })), (openMod.params || []).length ? /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'flex-start',
        gap: 20,
        flexWrap: 'wrap'
      }
    }, openMod.params.map(p => p.options ? /*#__PURE__*/React.createElement("div", {
      key: p.id,
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 6
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: lsLabel
    }, p.label), /*#__PURE__*/React.createElement("div", {
      role: "radiogroup",
      "aria-label": p.label,
      style: {
        display: 'flex',
        background: 'var(--dd-ink-0)',
        borderRadius: 'var(--radius-2, 4px)',
        padding: 2,
        gap: 2
      }
    }, p.options.map(o => /*#__PURE__*/React.createElement("button", {
      key: o,
      type: "button",
      role: "radio",
      "aria-checked": p.value === o,
      onClick: () => setP(p.id, o),
      style: {
        height: 22,
        padding: '0 8px',
        border: 'none',
        borderRadius: 3,
        cursor: 'pointer',
        background: p.value === o ? 'var(--dd-ink-5)' : 'transparent',
        boxShadow: p.value === o ? 'var(--shadow-cap)' : 'none',
        color: p.value === o ? 'var(--text-primary)' : 'var(--text-tertiary)',
        fontFamily: 'var(--font-value)',
        fontSize: 'var(--type-micro)',
        whiteSpace: 'nowrap',
        flex: 'none'
      }
    }, o)))) : /*#__PURE__*/React.createElement(__ds_scope.Knob, {
      key: p.id,
      size: "md",
      label: p.label,
      bipolar: p.bipolar,
      valueDisplay: "always",
      value: lsNorm(p, p.value),
      defaultValue: p.default != null ? lsNorm(p, p.default) : undefined,
      valueText: lsFmt(p),
      onChange: n => setP(p.id, lsDenorm(p, n))
    }))) : /*#__PURE__*/React.createElement("span", {
      style: {
        ...lsValue,
        color: 'var(--text-tertiary)'
      }
    }, openMod.src ? 'This source has no editable parameters.' : 'This module has no parameters.')));
  }), menu && (() => {
    const ml = local.find(x => x.id === menu.id);
    if (!ml) return null;
    return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", {
      onPointerDown: () => setMenu(null),
      style: {
        position: 'fixed',
        inset: 0,
        zIndex: 99
      }
    }), /*#__PURE__*/React.createElement(__ds_scope.ContextMenu, {
      title: "Output",
      x: menu.x,
      y: menu.y,
      width: 200,
      onClose: () => setMenu(null),
      items: outputs.map(o => ({
        label: o.name,
        shortcut: o.note,
        checked: (ml.output ?? outputs[0].id) === o.id,
        onSelect: () => {
          patchLayer(ml.id, x => ({
            ...x,
            output: o.id
          }));
          setMenu(null);
        }
      }))
    }));
  })());
}
Object.assign(__ds_scope, { LayerStack });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/LayerStack.jsx", error: String((e && e.message) || e) }); }

// components/display/OutputBusses.jsx
try { (() => {
const obLabel = {
  fontFamily: 'var(--font-ui)',
  fontSize: 'var(--type-micro)',
  fontWeight: 600,
  letterSpacing: '0.08em',
  textTransform: 'uppercase',
  color: 'var(--text-tertiary)'
};
const obValue = {
  fontFamily: 'var(--font-value)',
  fontSize: 'var(--type-micro)',
  color: 'var(--text-secondary)',
  whiteSpace: 'nowrap'
};
const obDb = v => v === 0 ? '0.0 dB' : `${v < 0 ? '−' : '+'}${Math.abs(v).toFixed(1)} dB`;

/**
 * Output busses: each maps to a plugin output channel pair (Main 1/2, 3/4…), shows what feeds it, a stereo meter, level and mute.
 * Layers and FX busses pick one of these as their output.
 */
function OutputBusses({
  busses = [],
  channelOptions = ['1/2', '3/4', '5/6', '7/8', '9/10', '11/12', '13/14', '15/16'],
  onChange,
  title = 'Output busses',
  style
}) {
  const [local, setLocal] = React.useState(busses);
  const ref = React.useRef(busses);
  React.useEffect(() => {
    setLocal(busses);
    ref.current = busses;
  }, [busses]);
  const [menu, setMenu] = React.useState(null);
  const patch = (id, p) => {
    const next = ref.current.map(b => b.id === id ? {
      ...b,
      ...p
    } : b);
    ref.current = next;
    setLocal(next);
    onChange && onChange(next, id);
  };
  const used = (ch, id) => local.some(b => b.id !== id && b.channels === ch);
  const grid = {
    display: 'grid',
    gridTemplateColumns: 'minmax(110px, 1fr) 84px minmax(120px, 2fr) 120px 56px 22px',
    alignItems: 'center',
    columnGap: 12
  };
  return /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      flexDirection: 'column',
      gap: 4,
      minWidth: 0,
      userSelect: 'none',
      ...style
    }
  }, /*#__PURE__*/React.createElement("div", {
    style: {
      display: 'flex',
      alignItems: 'baseline',
      gap: 10,
      minHeight: 18
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: obLabel
  }, title), /*#__PURE__*/React.createElement("span", {
    style: {
      ...obValue,
      color: 'var(--text-tertiary)',
      marginLeft: 'auto'
    }
  }, local.length, " of ", channelOptions.length, " stereo outputs")), /*#__PURE__*/React.createElement("div", {
    style: {
      ...grid,
      padding: '0 9px'
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: obLabel
  }, "Bus"), /*#__PURE__*/React.createElement("span", {
    style: obLabel
  }, "Plugin out"), /*#__PURE__*/React.createElement("span", {
    style: obLabel
  }, "Fed by"), /*#__PURE__*/React.createElement("span", {
    style: obLabel
  }, "Level"), /*#__PURE__*/React.createElement("span", null), /*#__PURE__*/React.createElement("span", null)), local.map(b => /*#__PURE__*/React.createElement("div", {
    key: b.id,
    style: {
      ...grid,
      padding: '6px 8px',
      background: 'var(--dd-ink-3)',
      border: '1px solid var(--border-hairline)',
      borderRadius: 'var(--radius-2, 4px)',
      opacity: b.muted ? 0.55 : 1
    }
  }, /*#__PURE__*/React.createElement("span", {
    style: {
      display: 'flex',
      alignItems: 'center',
      gap: 6,
      minWidth: 0
    }
  }, b.main && /*#__PURE__*/React.createElement("span", {
    style: {
      ...obLabel,
      padding: '1px 4px',
      background: 'var(--dd-ink-0)',
      borderRadius: 2,
      color: 'var(--text-secondary)'
    }
  }, "Main"), /*#__PURE__*/React.createElement("span", {
    style: {
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-label)',
      fontWeight: 600,
      color: 'var(--text-primary)',
      whiteSpace: 'nowrap'
    }
  }, b.name)), /*#__PURE__*/React.createElement("span", {
    onClickCapture: e => {
      const r = e.currentTarget.getBoundingClientRect();
      setMenu(menu && menu.id === b.id ? null : {
        id: b.id,
        x: r.left,
        y: r.bottom + 4
      });
    }
  }, /*#__PURE__*/React.createElement(__ds_scope.MenuButton, {
    compact: true,
    width: 84,
    value: `Out ${b.channels}`
  })), /*#__PURE__*/React.createElement("span", {
    style: {
      ...obValue,
      color: (b.feeds || []).length ? 'var(--text-secondary)' : 'var(--text-disabled)',
      overflow: 'hidden',
      textOverflow: 'ellipsis'
    },
    title: (b.feeds || []).join(', ')
  }, (b.feeds || []).length ? b.feeds.join(' · ') : 'Nothing routed'), /*#__PURE__*/React.createElement(__ds_scope.Meter, {
    orientation: "horizontal",
    compact: true,
    levels: b.levels || [-90, -90],
    length: 120,
    thickness: 4
  }), /*#__PURE__*/React.createElement("span", {
    style: {
      ...obValue,
      textAlign: 'right',
      color: 'var(--text-primary)'
    }
  }, obDb(b.level ?? 0)), /*#__PURE__*/React.createElement("button", {
    type: "button",
    "aria-label": `${b.muted ? 'Unmute' : 'Mute'} ${b.name}`,
    "aria-pressed": !!b.muted,
    onClick: () => patch(b.id, {
      muted: !b.muted
    }),
    style: {
      width: 22,
      height: 20,
      padding: 0,
      background: b.muted ? 'var(--dd-warn)' : 'var(--surface-control)',
      color: b.muted ? 'var(--text-on-accent)' : 'var(--text-secondary)',
      border: '1px solid var(--border-control)',
      borderRadius: 'var(--radius-2, 4px)',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--type-micro)',
      fontWeight: 700,
      cursor: 'pointer'
    }
  }, "M"))), menu && (() => {
    const b = local.find(x => x.id === menu.id);
    return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", {
      onPointerDown: () => setMenu(null),
      style: {
        position: 'fixed',
        inset: 0,
        zIndex: 99
      }
    }), /*#__PURE__*/React.createElement(__ds_scope.ContextMenu, {
      title: `${b.name} · plugin output`,
      x: menu.x,
      y: menu.y,
      width: 200,
      onClose: () => setMenu(null),
      items: channelOptions.map(ch => ({
        label: `Out ${ch}`,
        checked: b.channels === ch,
        shortcut: used(ch, b.id) ? 'shared' : undefined,
        onSelect: () => {
          patch(b.id, {
            channels: ch
          });
          setMenu(null);
        }
      }))
    }));
  })());
}
Object.assign(__ds_scope, { OutputBusses });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/display/OutputBusses.jsx", error: String((e && e.message) || e) }); }

// components/navigation/Tabs.jsx
try { (() => {
const focusRing = '0 0 0 2px var(--surface-panel), 0 0 0 4px var(--color-focus)';

/**
 * Panel tabs. Selected tab = primary text + 2px vermilion bar under the label (position + colour).
 * `badge` renders a small caps tag after the label (e.g. "BREAK" to scope a view to a patch type).
 */
function Tabs({
  items = [],
  value,
  onChange,
  compact = false,
  style
}) {
  const [focusIdx, setFocusIdx] = React.useState(-1);
  return /*#__PURE__*/React.createElement("div", {
    role: "tablist",
    style: {
      display: 'flex',
      alignItems: 'stretch',
      gap: 2,
      height: compact ? 26 : 32,
      ...style
    },
    onKeyDown: e => {
      const i = items.findIndex(t => t.id === value);
      if (e.key === 'ArrowRight') {
        const n = items[(i + 1) % items.length];
        onChange && onChange(n.id);
      }
      if (e.key === 'ArrowLeft') {
        const n = items[(i - 1 + items.length) % items.length];
        onChange && onChange(n.id);
      }
    }
  }, items.map((t, i) => {
    const sel = t.id === value;
    return /*#__PURE__*/React.createElement("button", {
      key: t.id,
      role: "tab",
      "aria-selected": sel,
      disabled: t.disabled,
      tabIndex: sel ? 0 : -1,
      onClick: () => onChange && onChange(t.id),
      onFocus: () => setFocusIdx(i),
      onBlur: () => setFocusIdx(-1),
      style: {
        position: 'relative',
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        padding: compact ? '0 8px' : '0 12px',
        background: 'transparent',
        border: 0,
        outline: 'none',
        cursor: t.disabled ? 'default' : 'pointer',
        borderRadius: 'var(--radius-1)',
        fontFamily: 'var(--font-ui)',
        fontSize: compact ? 'var(--type-label)' : 'var(--type-heading)',
        fontWeight: 600,
        letterSpacing: 'var(--tracking-heading)',
        textTransform: 'uppercase',
        color: t.disabled ? 'var(--text-disabled)' : sel ? 'var(--text-primary)' : 'var(--text-tertiary)',
        boxShadow: focusIdx === i ? focusRing : 'none'
      },
      onMouseEnter: e => {
        if (!sel && !t.disabled) e.currentTarget.style.color = 'var(--text-secondary)';
      },
      onMouseLeave: e => {
        if (!sel && !t.disabled) e.currentTarget.style.color = 'var(--text-tertiary)';
      }
    }, t.icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: t.icon,
      size: compact ? 14 : 16
    }), t.label, t.badge && /*#__PURE__*/React.createElement("span", {
      style: {
        fontSize: 10,
        letterSpacing: '0.08em',
        padding: '1px 4px',
        borderRadius: 2,
        border: '1px solid var(--border-strong)',
        color: 'var(--text-secondary)'
      }
    }, t.badge), /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: compact ? 8 : 12,
        right: compact ? 8 : 12,
        bottom: 0,
        height: 2,
        borderRadius: 1,
        background: sel ? 'var(--dd-vermilion)' : 'transparent'
      }
    }));
  }));
}

/** Segmented control: 2–5 mutually exclusive short options in one well. Selected = raised graphite face + primary text. */
function SegmentedControl({
  options = [],
  value,
  onChange,
  compact = false,
  disabled = false,
  hostAutomated = false,
  fullWidth = false,
  style
}) {
  const h = compact ? 22 : 24;
  return /*#__PURE__*/React.createElement("div", {
    role: "radiogroup",
    style: {
      display: fullWidth ? 'flex' : 'inline-flex',
      height: h,
      padding: 2,
      gap: 2,
      boxSizing: 'content-box',
      background: 'var(--surface-well)',
      borderRadius: 'var(--radius-2)',
      border: '1px solid var(--border-hairline)',
      ...style
    }
  }, options.map(o => {
    const opt = typeof o === 'string' ? {
      id: o,
      label: o
    } : o;
    const sel = opt.id === value;
    return /*#__PURE__*/React.createElement("button", {
      key: opt.id,
      role: "radio",
      "aria-checked": sel,
      disabled: disabled,
      title: opt.title,
      onClick: () => onChange && onChange(opt.id),
      style: {
        flex: fullWidth ? 1 : 'none',
        minWidth: compact ? 32 : 40,
        padding: '0 8px',
        display: 'flex',
        alignItems: 'center',
        justifyContent: 'center',
        gap: 5,
        border: 0,
        borderRadius: 3,
        cursor: disabled ? 'default' : 'pointer',
        outline: 'none',
        background: sel ? disabled ? 'var(--dd-ink-4)' : 'var(--dd-ink-5)' : 'transparent',
        boxShadow: sel && !disabled ? 'var(--shadow-cap), var(--highlight-cap)' : 'none',
        color: disabled ? 'var(--text-disabled)' : sel ? hostAutomated ? 'var(--dd-host)' : 'var(--text-primary)' : 'var(--text-tertiary)',
        fontFamily: 'var(--font-ui)',
        fontSize: compact ? 'var(--type-micro)' : 'var(--type-label)',
        fontWeight: 600,
        letterSpacing: 'var(--tracking-caps)',
        textTransform: 'uppercase'
      },
      onFocus: e => {
        e.currentTarget.style.boxShadow = focusRing;
      },
      onBlur: e => {
        e.currentTarget.style.boxShadow = sel && !disabled ? 'var(--shadow-cap), var(--highlight-cap)' : 'none';
      }
    }, opt.icon && /*#__PURE__*/React.createElement(__ds_scope.Icon, {
      name: opt.icon,
      size: 14
    }), opt.label);
  }));
}
Object.assign(__ds_scope, { Tabs, SegmentedControl });
})(); } catch (e) { __ds_ns.__errors.push({ path: "components/navigation/Tabs.jsx", error: String((e && e.message) || e) }); }

// tools/dev-bundle-fallback.js
try { (() => {
// Dev fallback: if _ds_bundle.js hasn't been compiled yet, transpile component sources in-browser.
// No-op when the compiled bundle is present.
(function () {
  var NS = 'DandrumDesignSystem_3c2eab';
  if (window[NS] || !window.Babel) return;
  var base = (document.currentScript.src || '').replace(/tools\/dev-bundle-fallback\.js.*$/, '');
  var files = ['components/icons/Icon.jsx', 'components/display/ModIndicator.jsx', 'components/controls/Knob.jsx', 'components/controls/Slider.jsx', 'components/controls/Button.jsx', 'components/controls/Toggle.jsx', 'components/controls/NumericField.jsx', 'components/navigation/Tabs.jsx', 'components/navigation/ContextMenu.jsx', 'components/display/PadCell.jsx', 'components/display/WaveformPanel.jsx', 'components/display/KeyMap.jsx', 'components/display/LayerStack.jsx', 'components/display/OutputBusses.jsx', 'components/display/Meter.jsx', 'components/display/ListRow.jsx', 'components/feedback/StatusMessage.jsx', 'components/layout/Panel.jsx'];
  var out = {};
  files.forEach(function (f) {
    var x = new XMLHttpRequest();
    x.open('GET', base + f, false);
    x.send();
    var src = x.responseText.replace(/^import .*$/mg, '');
    var names = [];
    src = src.replace(/export (function|const) (\w+)/g, function (_, k, n) {
      names.push(n);
      return k + ' ' + n;
    });
    var code = Babel.transform(src, {
      presets: ['react']
    }).code;
    var fn = new Function('React', 'NS', 'with (NS) {' + code + '\n;return {' + names.join(',') + '};}');
    Object.assign(out, fn(window.React, out));
  });
  window[NS] = out;
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "tools/dev-bundle-fallback.js", error: String((e && e.message) || e) }); }

// tools/tweaks-panel.jsx
try { (() => {
// @ds-adherence-ignore -- omelette starter scaffold (raw elements/hex/px by design)
// Copied omelette starter. Re-running copy_starter_component with this kind overwrites this file with the latest version (page content is unaffected).

/* BEGIN USAGE */
// tweaks-panel.jsx
// Reusable Tweaks shell + form-control helpers.
// Exports (to window): useTweaks, TweaksPanel, TweakSection, TweakRow, TweakSlider,
//   TweakToggle, TweakRadio, TweakSelect, TweakText, TweakNumber, TweakColor, TweakButton.
//
// Owns the host protocol (listens for __activate_edit_mode / __deactivate_edit_mode,
// posts __edit_mode_available / __edit_mode_set_keys / __edit_mode_dismissed) so
// individual prototypes don't re-roll it. Ships a consistent set of controls so you
// don't hand-draw <input type="range">, segmented radios, steppers, etc.
//
// Usage (in an HTML file that loads React + Babel):
//
//   const TWEAK_DEFAULTS = /*EDITMODE-BEGIN*/{
//     "primaryColor": "#D97757",
//     "palette": ["#D97757", "#29261b", "#f6f4ef"],
//     "fontSize": 16,
//     "density": "regular",
//     "dark": false
//   }/*EDITMODE-END*/;
//
//   function App() {
//     const [t, setTweak] = useTweaks(TWEAK_DEFAULTS);
//     return (
//       <div style={{ fontSize: t.fontSize, color: t.primaryColor }}>
//         Hello
//         <TweaksPanel>
//           <TweakSection label="Typography" />
//           <TweakSlider label="Font size" value={t.fontSize} min={10} max={32} unit="px"
//                        onChange={(v) => setTweak('fontSize', v)} />
//           <TweakRadio  label="Density" value={t.density}
//                        options={['compact', 'regular', 'comfy']}
//                        onChange={(v) => setTweak('density', v)} />
//           <TweakSection label="Theme" />
//           <TweakColor  label="Primary" value={t.primaryColor}
//                        options={['#D97757', '#2A6FDB', '#1F8A5B', '#7A5AE0']}
//                        onChange={(v) => setTweak('primaryColor', v)} />
//           <TweakColor  label="Palette" value={t.palette}
//                        options={[['#D97757', '#29261b', '#f6f4ef'],
//                                  ['#475569', '#0f172a', '#f1f5f9']]}
//                        onChange={(v) => setTweak('palette', v)} />
//           <TweakToggle label="Dark mode" value={t.dark}
//                        onChange={(v) => setTweak('dark', v)} />
//         </TweaksPanel>
//       </div>
//     );
//   }
//
// TweakRadio is the segmented control for 2–3 short options (auto-falls-back to
// TweakSelect past ~16/~10 chars per label); reach for TweakSelect directly when
// options are many or long. For color tweaks always curate 3-4 options rather than
// a free picker; an option can also be a whole 2–5 color palette (the stored value
// is the array). The Tweak* controls are a floor, not a ceiling — build custom
// controls inside the panel if a tweak calls for UI they don't cover.
/* END USAGE */
// ─────────────────────────────────────────────────────────────────────────────

const __TWEAKS_STYLE = `
  .twk-panel{position:fixed;right:16px;bottom:16px;z-index:2147483646;width:280px;
    max-height:calc(100vh - 32px);display:flex;flex-direction:column;
    transform:scale(var(--dc-inv-zoom,1));transform-origin:bottom right;
    background:rgba(250,249,247,.78);color:#29261b;
    -webkit-backdrop-filter:blur(24px) saturate(160%);backdrop-filter:blur(24px) saturate(160%);
    border:.5px solid rgba(255,255,255,.6);border-radius:14px;
    box-shadow:0 1px 0 rgba(255,255,255,.5) inset,0 12px 40px rgba(0,0,0,.18);
    font:11.5px/1.4 ui-sans-serif,system-ui,-apple-system,sans-serif;overflow:hidden}
  .twk-hd{display:flex;align-items:center;justify-content:space-between;
    padding:10px 8px 10px 14px;cursor:move;user-select:none}
  .twk-hd b{font-size:12px;font-weight:600;letter-spacing:.01em}
  .twk-x{appearance:none;border:0;background:transparent;color:rgba(41,38,27,.55);
    width:22px;height:22px;border-radius:6px;cursor:default;font-size:13px;line-height:1}
  .twk-x:hover{background:rgba(0,0,0,.06);color:#29261b}
  .twk-body{padding:2px 14px 14px;display:flex;flex-direction:column;gap:10px;
    overflow-y:auto;overflow-x:hidden;min-height:0;
    scrollbar-width:thin;scrollbar-color:rgba(0,0,0,.15) transparent}
  .twk-body::-webkit-scrollbar{width:8px}
  .twk-body::-webkit-scrollbar-track{background:transparent;margin:2px}
  .twk-body::-webkit-scrollbar-thumb{background:rgba(0,0,0,.15);border-radius:4px;
    border:2px solid transparent;background-clip:content-box}
  .twk-body::-webkit-scrollbar-thumb:hover{background:rgba(0,0,0,.25);
    border:2px solid transparent;background-clip:content-box}
  .twk-row{display:flex;flex-direction:column;gap:5px}
  .twk-row-h{flex-direction:row;align-items:center;justify-content:space-between;gap:10px}
  .twk-lbl{display:flex;justify-content:space-between;align-items:baseline;
    color:rgba(41,38,27,.72)}
  .twk-lbl>span:first-child{font-weight:500}
  .twk-val{color:rgba(41,38,27,.5);font-variant-numeric:tabular-nums}

  .twk-sect{font-size:10px;font-weight:600;letter-spacing:.06em;text-transform:uppercase;
    color:rgba(41,38,27,.45);padding:10px 0 0}
  .twk-sect:first-child{padding-top:0}

  .twk-field{appearance:none;box-sizing:border-box;width:100%;min-width:0;height:26px;padding:0 8px;
    border:.5px solid rgba(0,0,0,.1);border-radius:7px;
    background:rgba(255,255,255,.6);color:inherit;font:inherit;outline:none}
  .twk-field:focus{border-color:rgba(0,0,0,.25);background:rgba(255,255,255,.85)}
  select.twk-field{padding-right:22px;
    background-image:url("data:image/svg+xml;utf8,<svg xmlns='http://www.w3.org/2000/svg' width='10' height='6' viewBox='0 0 10 6'><path fill='rgba(0,0,0,.5)' d='M0 0h10L5 6z'/></svg>");
    background-repeat:no-repeat;background-position:right 8px center}

  .twk-slider{appearance:none;-webkit-appearance:none;width:100%;height:4px;margin:6px 0;
    border-radius:999px;background:rgba(0,0,0,.12);outline:none}
  .twk-slider::-webkit-slider-thumb{-webkit-appearance:none;appearance:none;
    width:14px;height:14px;border-radius:50%;background:#fff;
    border:.5px solid rgba(0,0,0,.12);box-shadow:0 1px 3px rgba(0,0,0,.2);cursor:default}
  .twk-slider::-moz-range-thumb{width:14px;height:14px;border-radius:50%;
    background:#fff;border:.5px solid rgba(0,0,0,.12);box-shadow:0 1px 3px rgba(0,0,0,.2);cursor:default}

  .twk-seg{position:relative;display:flex;padding:2px;border-radius:8px;
    background:rgba(0,0,0,.06);user-select:none}
  .twk-seg-thumb{position:absolute;top:2px;bottom:2px;border-radius:6px;
    background:rgba(255,255,255,.9);box-shadow:0 1px 2px rgba(0,0,0,.12);
    transition:left .15s cubic-bezier(.3,.7,.4,1),width .15s}
  .twk-seg.dragging .twk-seg-thumb{transition:none}
  .twk-seg button{appearance:none;position:relative;z-index:1;flex:1;border:0;
    background:transparent;color:inherit;font:inherit;font-weight:500;min-height:22px;
    border-radius:6px;cursor:default;padding:4px 6px;line-height:1.2;
    overflow-wrap:anywhere}

  .twk-toggle{position:relative;width:32px;height:18px;border:0;border-radius:999px;
    background:rgba(0,0,0,.15);transition:background .15s;cursor:default;padding:0}
  .twk-toggle[data-on="1"]{background:#34c759}
  .twk-toggle i{position:absolute;top:2px;left:2px;width:14px;height:14px;border-radius:50%;
    background:#fff;box-shadow:0 1px 2px rgba(0,0,0,.25);transition:transform .15s}
  .twk-toggle[data-on="1"] i{transform:translateX(14px)}

  .twk-num{display:flex;align-items:center;box-sizing:border-box;min-width:0;height:26px;padding:0 0 0 8px;
    border:.5px solid rgba(0,0,0,.1);border-radius:7px;background:rgba(255,255,255,.6)}
  .twk-num-lbl{font-weight:500;color:rgba(41,38,27,.6);cursor:ew-resize;
    user-select:none;padding-right:8px}
  .twk-num input{flex:1;min-width:0;height:100%;border:0;background:transparent;
    font:inherit;font-variant-numeric:tabular-nums;text-align:right;padding:0 8px 0 0;
    outline:none;color:inherit;-moz-appearance:textfield}
  .twk-num input::-webkit-inner-spin-button,.twk-num input::-webkit-outer-spin-button{
    -webkit-appearance:none;margin:0}
  .twk-num-unit{padding-right:8px;color:rgba(41,38,27,.45)}

  .twk-btn{appearance:none;height:26px;padding:0 12px;border:0;border-radius:7px;
    background:rgba(0,0,0,.78);color:#fff;font:inherit;font-weight:500;cursor:default}
  .twk-btn:hover{background:rgba(0,0,0,.88)}
  .twk-btn.secondary{background:rgba(0,0,0,.06);color:inherit}
  .twk-btn.secondary:hover{background:rgba(0,0,0,.1)}

  .twk-swatch{appearance:none;-webkit-appearance:none;width:56px;height:22px;
    border:.5px solid rgba(0,0,0,.1);border-radius:6px;padding:0;cursor:default;
    background:transparent;flex-shrink:0}
  .twk-swatch::-webkit-color-swatch-wrapper{padding:0}
  .twk-swatch::-webkit-color-swatch{border:0;border-radius:5.5px}
  .twk-swatch::-moz-color-swatch{border:0;border-radius:5.5px}

  .twk-chips{display:flex;gap:6px}
  .twk-chip{position:relative;appearance:none;flex:1;min-width:0;height:46px;
    padding:0;border:0;border-radius:6px;overflow:hidden;cursor:default;
    box-shadow:0 0 0 .5px rgba(0,0,0,.12),0 1px 2px rgba(0,0,0,.06);
    transition:transform .12s cubic-bezier(.3,.7,.4,1),box-shadow .12s}
  .twk-chip:hover{transform:translateY(-1px);
    box-shadow:0 0 0 .5px rgba(0,0,0,.18),0 4px 10px rgba(0,0,0,.12)}
  .twk-chip[data-on="1"]{box-shadow:0 0 0 1.5px rgba(0,0,0,.85),
    0 2px 6px rgba(0,0,0,.15)}
  .twk-chip>span{position:absolute;top:0;bottom:0;right:0;width:34%;
    display:flex;flex-direction:column;box-shadow:-1px 0 0 rgba(0,0,0,.1)}
  .twk-chip>span>i{flex:1;box-shadow:0 -1px 0 rgba(0,0,0,.1)}
  .twk-chip>span>i:first-child{box-shadow:none}
  .twk-chip svg{position:absolute;top:6px;left:6px;width:13px;height:13px;
    filter:drop-shadow(0 1px 1px rgba(0,0,0,.3))}
`;

// ── useTweaks ───────────────────────────────────────────────────────────────
// Single source of truth for tweak values. setTweak persists via the host
// (__edit_mode_set_keys → host rewrites the EDITMODE block on disk).
function useTweaks(defaults) {
  const [values, setValues] = React.useState(defaults);
  // Accepts either setTweak('key', value) or setTweak({ key: value, ... }) so a
  // useState-style call doesn't write a "[object Object]" key into the persisted
  // JSON block.
  const setTweak = React.useCallback((keyOrEdits, val) => {
    const edits = typeof keyOrEdits === 'object' && keyOrEdits !== null ? keyOrEdits : {
      [keyOrEdits]: val
    };
    setValues(prev => ({
      ...prev,
      ...edits
    }));
    window.parent.postMessage({
      type: '__edit_mode_set_keys',
      edits
    }, '*');
    // Same-window signal so in-page listeners (deck-stage rail thumbnails)
    // can react — the parent message only reaches the host, not peers.
    window.dispatchEvent(new CustomEvent('tweakchange', {
      detail: edits
    }));
  }, []);
  return [values, setTweak];
}

// ── TweaksPanel ─────────────────────────────────────────────────────────────
// Floating shell. Registers the protocol listener BEFORE announcing
// availability — if the announce ran first, the host's activate could land
// before our handler exists and the toolbar toggle would silently no-op.
// The close button posts __edit_mode_dismissed so the host's toolbar toggle
// flips off in lockstep; the host echoes __deactivate_edit_mode back which
// is what actually hides the panel.
function TweaksPanel({
  title = 'Tweaks',
  children
}) {
  const [open, setOpen] = React.useState(false);
  const dragRef = React.useRef(null);
  const offsetRef = React.useRef({
    x: 16,
    y: 16
  });
  const PAD = 16;
  const clampToViewport = React.useCallback(() => {
    const panel = dragRef.current;
    if (!panel) return;
    const w = panel.offsetWidth,
      h = panel.offsetHeight;
    const maxRight = Math.max(PAD, window.innerWidth - w - PAD);
    const maxBottom = Math.max(PAD, window.innerHeight - h - PAD);
    offsetRef.current = {
      x: Math.min(maxRight, Math.max(PAD, offsetRef.current.x)),
      y: Math.min(maxBottom, Math.max(PAD, offsetRef.current.y))
    };
    panel.style.right = offsetRef.current.x + 'px';
    panel.style.bottom = offsetRef.current.y + 'px';
  }, []);
  React.useEffect(() => {
    if (!open) return;
    clampToViewport();
    if (typeof ResizeObserver === 'undefined') {
      window.addEventListener('resize', clampToViewport);
      return () => window.removeEventListener('resize', clampToViewport);
    }
    const ro = new ResizeObserver(clampToViewport);
    ro.observe(document.documentElement);
    return () => ro.disconnect();
  }, [open, clampToViewport]);
  React.useEffect(() => {
    const onMsg = e => {
      const t = e?.data?.type;
      if (t === '__activate_edit_mode') setOpen(true);else if (t === '__deactivate_edit_mode') setOpen(false);
    };
    window.addEventListener('message', onMsg);
    window.parent.postMessage({
      type: '__edit_mode_available'
    }, '*');
    return () => window.removeEventListener('message', onMsg);
  }, []);
  const dismiss = () => {
    setOpen(false);
    window.parent.postMessage({
      type: '__edit_mode_dismissed'
    }, '*');
  };
  const onDragStart = e => {
    const panel = dragRef.current;
    if (!panel) return;
    const r = panel.getBoundingClientRect();
    const sx = e.clientX,
      sy = e.clientY;
    const startRight = window.innerWidth - r.right;
    const startBottom = window.innerHeight - r.bottom;
    const move = ev => {
      offsetRef.current = {
        x: startRight - (ev.clientX - sx),
        y: startBottom - (ev.clientY - sy)
      };
      clampToViewport();
    };
    const up = () => {
      window.removeEventListener('mousemove', move);
      window.removeEventListener('mouseup', up);
    };
    window.addEventListener('mousemove', move);
    window.addEventListener('mouseup', up);
  };

  // data-om-starter: inert presence marker — Claude Design's starter-usage
  // probe reads it. The closed panel renders nothing, so the marker rides
  // the <html> element as an attribute instead of a rendered node — zero
  // elements added, so page CSS (even structural selectors like
  // :nth-child) can never observe it. It records that the page WIRES a
  // tweaks panel, whether or not the panel is open. Keep this effect.
  React.useEffect(() => {
    document.documentElement.setAttribute('data-om-starter', 'tweaks-panel');
    return () => document.documentElement.removeAttribute('data-om-starter');
  }, []);
  if (!open) return null;
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("style", null, __TWEAKS_STYLE), /*#__PURE__*/React.createElement("div", {
    ref: dragRef,
    className: "twk-panel",
    "data-omelette-chrome": "",
    style: {
      right: offsetRef.current.x,
      bottom: offsetRef.current.y
    }
  }, /*#__PURE__*/React.createElement("div", {
    className: "twk-hd",
    onMouseDown: onDragStart
  }, /*#__PURE__*/React.createElement("b", null, title), /*#__PURE__*/React.createElement("button", {
    className: "twk-x",
    "aria-label": "Close tweaks",
    onMouseDown: e => e.stopPropagation(),
    onClick: dismiss
  }, "\u2715")), /*#__PURE__*/React.createElement("div", {
    className: "twk-body"
  }, children)));
}

// ── Layout helpers ──────────────────────────────────────────────────────────

function TweakSection({
  label,
  children
}) {
  return /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("div", {
    className: "twk-sect"
  }, label), children);
}
function TweakRow({
  label,
  value,
  children,
  inline = false
}) {
  return /*#__PURE__*/React.createElement("div", {
    className: inline ? 'twk-row twk-row-h' : 'twk-row'
  }, /*#__PURE__*/React.createElement("div", {
    className: "twk-lbl"
  }, /*#__PURE__*/React.createElement("span", null, label), value != null && /*#__PURE__*/React.createElement("span", {
    className: "twk-val"
  }, value)), children);
}

// ── Controls ────────────────────────────────────────────────────────────────

function TweakSlider({
  label,
  value,
  min = 0,
  max = 100,
  step = 1,
  unit = '',
  onChange
}) {
  return /*#__PURE__*/React.createElement(TweakRow, {
    label: label,
    value: `${value}${unit}`
  }, /*#__PURE__*/React.createElement("input", {
    type: "range",
    className: "twk-slider",
    min: min,
    max: max,
    step: step,
    value: value,
    onChange: e => onChange(Number(e.target.value))
  }));
}
function TweakToggle({
  label,
  value,
  onChange
}) {
  return /*#__PURE__*/React.createElement("div", {
    className: "twk-row twk-row-h"
  }, /*#__PURE__*/React.createElement("div", {
    className: "twk-lbl"
  }, /*#__PURE__*/React.createElement("span", null, label)), /*#__PURE__*/React.createElement("button", {
    type: "button",
    className: "twk-toggle",
    "data-on": value ? '1' : '0',
    role: "switch",
    "aria-checked": !!value,
    onClick: () => onChange(!value)
  }, /*#__PURE__*/React.createElement("i", null)));
}
function TweakRadio({
  label,
  value,
  options,
  onChange
}) {
  const trackRef = React.useRef(null);
  const [dragging, setDragging] = React.useState(false);
  // The active value is read by pointer-move handlers attached for the lifetime
  // of a drag — ref it so a stale closure doesn't fire onChange for every move.
  const valueRef = React.useRef(value);
  valueRef.current = value;

  // Segments wrap mid-word once per-segment width runs out. The track is
  // ~248px (280 panel − 28 body pad − 4 seg pad), each button loses 12px
  // to its own padding, and 11.5px system-ui averages ~6.3px/char — so 2
  // options fit ~16 chars each, 3 fit ~10. Past that (or >3 options), fall
  // back to a dropdown rather than wrap.
  const labelLen = o => String(typeof o === 'object' ? o.label : o).length;
  const maxLen = options.reduce((m, o) => Math.max(m, labelLen(o)), 0);
  const fitsAsSegments = maxLen <= ({
    2: 16,
    3: 10
  }[options.length] ?? 0);
  if (!fitsAsSegments) {
    // <select> emits strings — map back to the original option value so the
    // fallback stays type-preserving (numbers, booleans) like the segment path.
    const resolve = s => {
      const m = options.find(o => String(typeof o === 'object' ? o.value : o) === s);
      return m === undefined ? s : typeof m === 'object' ? m.value : m;
    };
    return /*#__PURE__*/React.createElement(TweakSelect, {
      label: label,
      value: value,
      options: options,
      onChange: s => onChange(resolve(s))
    });
  }
  const opts = options.map(o => typeof o === 'object' ? o : {
    value: o,
    label: o
  });
  const idx = Math.max(0, opts.findIndex(o => o.value === value));
  const n = opts.length;
  const segAt = clientX => {
    const r = trackRef.current.getBoundingClientRect();
    const inner = r.width - 4;
    const i = Math.floor((clientX - r.left - 2) / inner * n);
    return opts[Math.max(0, Math.min(n - 1, i))].value;
  };
  const onPointerDown = e => {
    setDragging(true);
    const v0 = segAt(e.clientX);
    if (v0 !== valueRef.current) onChange(v0);
    const move = ev => {
      if (!trackRef.current) return;
      const v = segAt(ev.clientX);
      if (v !== valueRef.current) onChange(v);
    };
    const up = () => {
      setDragging(false);
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  return /*#__PURE__*/React.createElement(TweakRow, {
    label: label
  }, /*#__PURE__*/React.createElement("div", {
    ref: trackRef,
    role: "radiogroup",
    onPointerDown: onPointerDown,
    className: dragging ? 'twk-seg dragging' : 'twk-seg'
  }, /*#__PURE__*/React.createElement("div", {
    className: "twk-seg-thumb",
    style: {
      left: `calc(2px + ${idx} * (100% - 4px) / ${n})`,
      width: `calc((100% - 4px) / ${n})`
    }
  }), opts.map(o => /*#__PURE__*/React.createElement("button", {
    key: o.value,
    type: "button",
    role: "radio",
    "aria-checked": o.value === value
  }, o.label))));
}
function TweakSelect({
  label,
  value,
  options,
  onChange
}) {
  return /*#__PURE__*/React.createElement(TweakRow, {
    label: label
  }, /*#__PURE__*/React.createElement("select", {
    className: "twk-field",
    value: value,
    onChange: e => onChange(e.target.value)
  }, options.map(o => {
    const v = typeof o === 'object' ? o.value : o;
    const l = typeof o === 'object' ? o.label : o;
    return /*#__PURE__*/React.createElement("option", {
      key: v,
      value: v
    }, l);
  })));
}
function TweakText({
  label,
  value,
  placeholder,
  onChange
}) {
  return /*#__PURE__*/React.createElement(TweakRow, {
    label: label
  }, /*#__PURE__*/React.createElement("input", {
    className: "twk-field",
    type: "text",
    value: value,
    placeholder: placeholder,
    onChange: e => onChange(e.target.value)
  }));
}
function TweakNumber({
  label,
  value,
  min,
  max,
  step = 1,
  unit = '',
  onChange
}) {
  const clamp = n => {
    if (min != null && n < min) return min;
    if (max != null && n > max) return max;
    return n;
  };
  const startRef = React.useRef({
    x: 0,
    val: 0
  });
  const onScrubStart = e => {
    e.preventDefault();
    startRef.current = {
      x: e.clientX,
      val: value
    };
    const decimals = (String(step).split('.')[1] || '').length;
    const move = ev => {
      const dx = ev.clientX - startRef.current.x;
      const raw = startRef.current.val + dx * step;
      const snapped = Math.round(raw / step) * step;
      onChange(clamp(Number(snapped.toFixed(decimals))));
    };
    const up = () => {
      window.removeEventListener('pointermove', move);
      window.removeEventListener('pointerup', up);
    };
    window.addEventListener('pointermove', move);
    window.addEventListener('pointerup', up);
  };
  return /*#__PURE__*/React.createElement("div", {
    className: "twk-num"
  }, /*#__PURE__*/React.createElement("span", {
    className: "twk-num-lbl",
    onPointerDown: onScrubStart
  }, label), /*#__PURE__*/React.createElement("input", {
    type: "number",
    value: value,
    min: min,
    max: max,
    step: step,
    onChange: e => onChange(clamp(Number(e.target.value)))
  }), unit && /*#__PURE__*/React.createElement("span", {
    className: "twk-num-unit"
  }, unit));
}

// Relative-luminance contrast pick — checkmarks drawn over a swatch need to
// read on both #111 and #fafafa without per-option configuration. Hex input
// only (#rgb / #rrggbb); named or rgb()/hsl() colors fall through to "light".
function __twkIsLight(hex) {
  const h = String(hex).replace('#', '');
  const x = h.length === 3 ? h.replace(/./g, c => c + c) : h.padEnd(6, '0');
  const n = parseInt(x.slice(0, 6), 16);
  if (Number.isNaN(n)) return true;
  const r = n >> 16 & 255,
    g = n >> 8 & 255,
    b = n & 255;
  return r * 299 + g * 587 + b * 114 > 148000;
}
const __TwkCheck = ({
  light
}) => /*#__PURE__*/React.createElement("svg", {
  viewBox: "0 0 14 14",
  "aria-hidden": "true"
}, /*#__PURE__*/React.createElement("path", {
  d: "M3 7.2 5.8 10 11 4.2",
  fill: "none",
  strokeWidth: "2.2",
  strokeLinecap: "round",
  strokeLinejoin: "round",
  stroke: light ? 'rgba(0,0,0,.78)' : '#fff'
}));

// TweakColor — curated color/palette picker. Each option is either a single
// hex string or an array of 1-5 hex strings; the card adapts — a lone color
// renders solid, a palette renders colors[0] as the hero (left ~2/3) with the
// rest stacked in a sharp column on the right. onChange emits the
// option in the shape it was passed (string stays string, array stays array).
// Without options it falls back to the native color input for back-compat.
function TweakColor({
  label,
  value,
  options,
  onChange
}) {
  if (!options || !options.length) {
    return /*#__PURE__*/React.createElement("div", {
      className: "twk-row twk-row-h"
    }, /*#__PURE__*/React.createElement("div", {
      className: "twk-lbl"
    }, /*#__PURE__*/React.createElement("span", null, label)), /*#__PURE__*/React.createElement("input", {
      type: "color",
      className: "twk-swatch",
      value: value,
      onChange: e => onChange(e.target.value)
    }));
  }
  // Native <input type=color> emits lowercase hex per the HTML spec, so
  // compare case-insensitively. String() guards JSON.stringify(undefined),
  // which returns the primitive undefined (no .toLowerCase).
  const key = o => String(JSON.stringify(o)).toLowerCase();
  const cur = key(value);
  return /*#__PURE__*/React.createElement(TweakRow, {
    label: label
  }, /*#__PURE__*/React.createElement("div", {
    className: "twk-chips",
    role: "radiogroup"
  }, options.map((o, i) => {
    const colors = Array.isArray(o) ? o : [o];
    const [hero, ...rest] = colors;
    const sup = rest.slice(0, 4);
    const on = key(o) === cur;
    return /*#__PURE__*/React.createElement("button", {
      key: i,
      type: "button",
      className: "twk-chip",
      role: "radio",
      "aria-checked": on,
      "data-on": on ? '1' : '0',
      "aria-label": colors.join(', '),
      title: colors.join(' · '),
      style: {
        background: hero
      },
      onClick: () => onChange(o)
    }, sup.length > 0 && /*#__PURE__*/React.createElement("span", null, sup.map((c, j) => /*#__PURE__*/React.createElement("i", {
      key: j,
      style: {
        background: c
      }
    }))), on && /*#__PURE__*/React.createElement(__TwkCheck, {
      light: __twkIsLight(hero)
    }));
  })));
}
function TweakButton({
  label,
  onClick,
  secondary = false
}) {
  return /*#__PURE__*/React.createElement("button", {
    type: "button",
    className: secondary ? 'twk-btn secondary' : 'twk-btn',
    onClick: onClick
  }, label);
}
Object.assign(window, {
  useTweaks,
  TweaksPanel,
  TweakSection,
  TweakRow,
  TweakSlider,
  TweakToggle,
  TweakRadio,
  TweakSelect,
  TweakText,
  TweakNumber,
  TweakColor,
  TweakButton
});
})(); } catch (e) { __ds_ns.__errors.push({ path: "tools/tweaks-panel.jsx", error: String((e && e.message) || e) }); }

// ui_kits/das-sampler/App.jsx
try { (() => {
// Das Sampler — app shell, state, playback simulation.
(() => {
  const {
    ContextMenu,
    SegmentedControl,
    Button,
    Toggle
  } = window.DandrumDesignSystem_3c2eab;
  const D = window.SAMPLER_DATA;
  function pickRegion(pad, vel, counters, note) {
    const layerIdx = pad.layers.findIndex(l => vel * 127 >= l.vel[0] && vel * 127 <= l.vel[1] + 0.99);
    const li = layerIdx < 0 ? 0 : layerIdx;
    const layer = pad.layers[li];
    let alt = 0;
    if (layer.regions.length > 1) {
      const c = counters[note] = (counters[note] || 0) + 1;
      alt = layer.policy === 'Weighted' ? c * 7919 % 16 / 16 < 1 / 1.6 ? 0 : 1 : c % layer.regions.length;
    }
    let flat = 0;
    for (let i = 0; i < li; i++) flat += pad.layers[i].regions.length;
    return {
      layer: li,
      alt,
      flatIdx: flat + alt,
      region: layer.regions[alt]
    };
  }
  function SamplerApp() {
    const init = window.SAMPLER_INIT || {};
    const [layout, setLayout] = React.useState(init.layout || 'full');
    const [patchKind, setPatchKind] = React.useState(init.patch || 'kit');
    const patch = patchKind === 'kit' ? D.kit : D.breakPatch;
    const [selected, setSelected] = React.useState(init.selected ?? 38);
    const [regionIdx, setRegionIdx] = React.useState(1);
    const [view, setView] = React.useState(init.view || 'region');
    const [slice, setSlice] = React.useState(2);
    const [values, setValues] = React.useState({
      pitch: 0.5,
      start: 0.08,
      level: 0.8,
      pan: 0.5,
      variation: 0.35
    });
    const [mods, setMods] = React.useState({
      pitch: [{
        slot: 'A',
        source: 'Velocity',
        depth: 0.12
      }, {
        slot: 'B',
        source: 'Random per hit',
        depth: -0.08
      }],
      level: [{
        slot: 'A',
        source: 'Velocity',
        depth: 0.25
      }],
      variation: [{
        slot: 'B',
        source: 'Random per hit',
        depth: 0.3
      }]
    });
    const [liveMod, setLiveMod] = React.useState({});
    const [activity, setActivity] = React.useState({});
    const [cursor, setCursor] = React.useState(null);
    const [menu, setMenu] = React.useState(null);
    const [assigning, setAssigning] = React.useState(null);
    const [hostParam, setHostParam] = React.useState(init.host || null);
    const [hostFlash, setHostFlash] = React.useState(null);
    const [missing, setMissing] = React.useState(!!init.missing);
    const [reloading, setReloading] = React.useState(false);
    const [hoverId, setHoverId] = React.useState(null);
    const [pattern, setPattern] = React.useState(!!init.pattern);
    const [chokeFlash, setChokeFlash] = React.useState(false);
    const voicesRef = React.useRef([]);
    const counters = React.useRef({});
    const selRef = React.useRef(selected);
    selRef.current = selected;
    const valRef = React.useRef(values);
    valRef.current = values;
    const compact = layout === 'compact';
    const winRef = React.useRef(null);
    const W = compact ? 820 : 1200,
      H = compact ? 560 : 800;
    const [scale, setScale] = React.useState(1);
    React.useEffect(() => {
      const f = () => setScale(Math.min(1, (window.innerWidth - 8) / W));
      f();
      window.addEventListener('resize', f);
      return () => window.removeEventListener('resize', f);
    }, [W]);
    const toLocal = (cx, cy) => {
      const r = winRef.current.getBoundingClientRect();
      return {
        x: (cx - r.left) / scale,
        y: (cy - r.top) / scale
      };
    };

    // open initial menu for the "menu" state screen
    React.useEffect(() => {
      if (init.menu) {
        const t = setTimeout(() => {
          const el = document.querySelector('[aria-label="' + init.menuLabel + '"]');
          const r = el ? el.getBoundingClientRect() : {
            left: 400,
            bottom: 500,
            width: 0
          };
          const p = toLocal(r.right + 4, r.top);
          setMenu({
            param: init.menu,
            x: p.x,
            y: Math.min(p.y, H - 400),
            sub: false
          });
        }, 300);
        return () => clearTimeout(t);
      }
    }, []);
    const trigger = React.useCallback((note, vel) => {
      const now = performance.now();
      if (patch.kind === 'break') {
        const i = note - 36;
        if (i < 0 || i >= patch.slices.length) return;
        const len = ((patch.slices[i + 1] ?? 1) - patch.slices[i]) * patch.len;
        voicesRef.current = voicesRef.current.filter(v => false);
        voicesRef.current.push({
          note,
          t0: now,
          len,
          from: patch.slices[i],
          to: patch.slices[i + 1] ?? 1,
          vel
        });
        setSlice(i);
        setActivity(a => ({
          ...a,
          [note]: {
            level: 1,
            velocity: vel
          }
        }));
        setLiveMod({
          A: vel,
          B: Math.random()
        });
        return;
      }
      const pad = patch.pads[note];
      if (!pad) return;
      let pick = pickRegion(pad, vel, counters.current, note);
      if (missing && pick.region.id === 'hat-open-b') pick = {
        ...pick,
        alt: 0,
        flatIdx: 0,
        region: pad.layers[0].regions[0]
      };
      const ratio = Math.pow(2, (valRef.current.pitch - 0.5) * 2);
      const r = pick.region;
      const len = (r.end - r.start) * r.len / ratio;
      // choke
      if (pad.choke) {
        Object.entries(patch.pads).forEach(([n, p]) => {
          if (Number(n) !== note && p.choke === pad.choke) {
            voicesRef.current = voicesRef.current.filter(v => v.note !== Number(n));
            setActivity(a => a[n] && a[n].level > 0.02 ? {
              ...a,
              [n]: {
                ...a[n],
                level: 0,
                choked: true
              }
            } : a);
            setChokeFlash(true);
            setTimeout(() => setChokeFlash(false), 180);
          }
        });
      }
      voicesRef.current = voicesRef.current.filter(v => v.note !== note || voicesRef.current.filter(x => x.note === note).length < pad.voices);
      voicesRef.current.push({
        note,
        t0: now,
        len,
        from: r.start + valRef.current.start * (r.end - r.start) * 0.3,
        to: r.end,
        vel
      });
      setActivity(a => ({
        ...a,
        [note]: {
          level: 1,
          velocity: vel,
          layer: pad.layers.length > 1 ? pick.layer : -1,
          alt: pick.alt,
          choked: false,
          region: r.id
        }
      }));
      setLiveMod({
        A: vel,
        B: (note * 31 + (counters.current[note] || 0) * 17) % 100 / 100
      });
      if (note === selRef.current) setRegionIdx(pick.flatIdx);
    }, [patch, missing]);

    // frame loop: decay pad levels, move cursor
    React.useEffect(() => {
      let raf;
      const tick = () => {
        const now = performance.now();
        voicesRef.current = voicesRef.current.filter(v => now - v.t0 < v.len);
        setActivity(a => {
          let changed = false;
          const n = {
            ...a
          };
          for (const k of Object.keys(n)) {
            const vs = voicesRef.current.filter(v => v.note === Number(k));
            const lvl = vs.length ? Math.max(...vs.map(v => 1 - (now - v.t0) / v.len)) : 0;
            if (Math.abs((n[k].level || 0) - lvl) > 0.005) {
              n[k] = {
                ...n[k],
                level: lvl
              };
              changed = true;
            }
            if (n[k].choked && now % 1000 < 20) {/* keep */}
          }
          return changed ? n : a;
        });
        const sv = voicesRef.current.filter(v => patch.kind === 'break' || v.note === selRef.current).slice(-1)[0];
        setCursor(sv ? sv.from + (sv.to - sv.from) * ((now - sv.t0) / sv.len) : null);
        if (!voicesRef.current.length) setLiveMod(m => m.A != null || m.B != null ? {} : m);
        raf = requestAnimationFrame(tick);
      };
      raf = requestAnimationFrame(tick);
      return () => cancelAnimationFrame(raf);
    }, [patch]);

    // clear choked flag after a moment
    React.useEffect(() => {
      const ch = Object.entries(activity).filter(([, a]) => a.choked);
      if (!ch.length) return;
      const t = setTimeout(() => setActivity(a => {
        const n = {
          ...a
        };
        ch.forEach(([k]) => {
          n[k] = {
            ...n[k],
            choked: false
          };
        });
        return n;
      }), 500);
      return () => clearTimeout(t);
    }, [activity]);

    // demo pattern
    React.useEffect(() => {
      if (!pattern) return;
      const seq = patch.kind === 'break' ? [36, 37, 38, 39, 40, 41, 42, 43].map(n => [n]) : [[36, 42], [42], [38, 42], [42], [36, 42], [36, 46], [38, 42], [46]];
      let i = 0;
      const id = setInterval(() => {
        seq[i % seq.length].forEach(n => trigger(n, n === 38 ? i % 4 === 2 ? 0.95 : 0.6 : n === 42 ? 0.55 + i % 2 * 0.2 : 0.85));
        i++;
      }, 136);
      return () => clearInterval(id);
    }, [pattern, trigger, patch]);

    // simulated host automation
    React.useEffect(() => {
      if (!hostParam) return;
      let t = 0;
      const id = setInterval(() => {
        t += 0.05;
        setValues(v => ({
          ...v,
          [hostParam]: hostParam === 'pan' ? 0.5 + Math.sin(t) * 0.3 : 0.62 + Math.sin(t) * 0.18
        }));
        setHostFlash(hostParam);
      }, 50);
      return () => {
        clearInterval(id);
        setHostFlash(null);
      };
    }, [hostParam]);
    React.useEffect(() => {
      const k = e => {
        if (e.key === 'Escape') {
          setAssigning(null);
          setMenu(null);
        }
      };
      window.addEventListener('keydown', k);
      return () => window.removeEventListener('keydown', k);
    }, []);
    const pad = patch.kind === 'kit' ? patch.pads[selected] : null;
    const regions = pad ? pad.layers.flatMap(l => l.regions) : [];
    const region = regions[Math.min(regionIdx, regions.length - 1)];
    const missingRegionId = missing ? 'hat-open-b' : null;
    const status = reloading ? {
      kind: 'busy',
      title: 'Preparing patch…',
      text: 'loading sample regions',
      short: 'Preparing…'
    } : missing ? {
      kind: 'error',
      title: '1 sample file missing',
      text: 'hat_open_b.wav · Open Hat alternate B will not play',
      short: '1 file missing'
    } : {
      kind: 'ok',
      title: 'Patch loaded',
      text: patch.kind === 'kit' ? '4 sounds · 7 sample regions' : '1 sample · 8 slices',
      short: 'Loaded'
    };
    const onReload = () => {
      setReloading(true);
      voicesRef.current = [];
      setTimeout(() => {
        setReloading(false);
      }, 900);
    };
    const setDepth = (param, slot, d) => setMods(m => ({
      ...m,
      [param]: m[param].map(a => a.slot === slot ? {
        ...a,
        depth: d
      } : a)
    }));
    const remove = (param, slot) => setMods(m => ({
      ...m,
      [param]: (m[param] || []).filter(a => a.slot !== slot)
    }));
    const add = (param, s) => setMods(m => ({
      ...m,
      [param]: [...(m[param] || []).filter(a => a.slot !== s.slot), {
        slot: s.slot,
        source: s.name,
        depth: 0.25
      }]
    }));
    const startAssign = (param, s) => {
      add(param, s);
      setAssigning({
        slot: s.slot,
        name: s.name
      });
      setMenu(null);
    };
    const onMenu = (param, e) => {
      const p = toLocal(e.clientX, e.clientY);
      setMenu({
        param,
        x: Math.min(p.x, W - 256),
        y: Math.min(p.y, H - 380),
        sub: false
      });
    };
    const menuSpec = menu && buildModMenu({
      param: menu.param,
      value: values[menu.param],
      mods,
      setDepth,
      remove,
      startAssign,
      hostParam,
      reset: p => setValues(v => ({
        ...v,
        [p]: D.liveParams.find(x => x.id === p).def
      })),
      close: () => setMenu(null),
      sub: menu.sub,
      setSub: s => setMenu(m => ({
        ...m,
        sub: s
      }))
    });
    const levels = [-60 + Math.max(0, ...Object.values(activity).map(a => a.level || 0)) * 52, -60 + Math.max(0, ...Object.values(activity).map(a => a.level || 0)) * 49];
    const startMod = (mods.start || [])[0];
    return /*#__PURE__*/React.createElement("div", {
      style: {
        height: (H + 80) * scale,
        width: W * scale,
        display: 'flex',
        justifyContent: 'center'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        alignItems: 'center',
        gap: 12,
        transform: `scale(${scale})`,
        transformOrigin: 'top center',
        flex: 'none'
      }
    }, /*#__PURE__*/React.createElement("div", {
      ref: winRef,
      "data-screen-label": compact ? 'Compact' : 'Main',
      style: {
        width: W,
        height: H,
        display: 'flex',
        flexDirection: 'column',
        background: 'var(--surface-window)',
        border: '1px solid #000',
        boxShadow: '0 20px 60px rgba(0,0,0,0.5)',
        overflow: 'hidden',
        position: 'relative',
        transform: 'translateZ(0)',
        flex: 'none'
      },
      onClick: () => menu && setMenu(null)
    }, /*#__PURE__*/React.createElement(SamplerHeader, {
      patch: patch,
      status: status,
      compact: compact,
      onLayout: setLayout,
      onReload: onReload,
      levels: levels
    }), missing && !compact && /*#__PURE__*/React.createElement("div", {
      style: {
        padding: '4px 4px 0'
      }
    }, /*#__PURE__*/React.createElement(window.DandrumDesignSystem_3c2eab.StatusMessage, {
      kind: "error",
      compact: true,
      title: "Open Hat \xB7 alternate B unavailable",
      action: /*#__PURE__*/React.createElement("div", {
        style: {
          display: 'flex',
          gap: 6
        }
      }, /*#__PURE__*/React.createElement(Button, {
        size: "sm",
        icon: "folder"
      }, "Locate file\u2026"), /*#__PURE__*/React.createElement(Button, {
        size: "sm",
        variant: "primary",
        icon: "reload",
        onClick: () => {
          setMissing(false);
          onReload();
        }
      }, "Reload Patch"))
    }, "hat_open_b.wav was not found. Open Hat plays alternate A only until the file is restored and the patch is reloaded.")), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        minHeight: 0,
        display: 'grid',
        gridTemplateColumns: compact ? '240px minmax(0,1fr)' : '360px minmax(0,1fr)',
        gap: 4,
        padding: 4
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
        minHeight: 0
      }
    }, /*#__PURE__*/React.createElement(PadGrid, {
      patch: patch,
      selected: selected,
      onSelect: n => {
        setSelected(n);
        setRegionIdx(0);
      },
      onTrigger: trigger,
      activity: activity,
      compact: compact,
      chokeFlash: chokeFlash,
      missing: missing
    }), /*#__PURE__*/React.createElement(PadDetails, {
      note: selected,
      pad: pad,
      regionIdx: Math.min(regionIdx, Math.max(0, regions.length - 1)),
      onRegion: setRegionIdx,
      activeRegion: activity[selected] && activity[selected].level > 0.02 ? activity[selected].region : null,
      compact: compact,
      missingRegionId: missingRegionId,
      patch: patch
    })), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 4,
        minHeight: 0
      }
    }, /*#__PURE__*/React.createElement(WaveView, {
      patch: patch,
      view: view,
      onView: setView,
      pad: pad,
      region: region,
      regionIdx: Math.min(regionIdx, Math.max(0, regions.length - 1)),
      onRegion: setRegionIdx,
      cursor: cursor,
      startOffset: values.start,
      hostStart: hostFlash === 'start',
      startMod: startMod ? {
        depth: startMod.depth,
        color: 'var(--dd-mod-' + startMod.slot.toLowerCase() + ')'
      } : null,
      compact: compact,
      slice: slice,
      onSlice: i => {
        setSlice(i);
        trigger(36 + i, 0.8);
      },
      onLoadBreak: () => {
        setPatchKind('break');
        setSelected(38);
        setView('slices');
        onReload();
      },
      onLoadKit: () => {
        setPatchKind('kit');
        setSelected(38);
        setView('region');
        onReload();
      },
      missingRegion: missing && region && region.id === 'hat-open-b'
    }), /*#__PURE__*/React.createElement(LiveControls, {
      values: values,
      onChange: (id, v) => setValues(x => ({
        ...x,
        [id]: v
      })),
      mods: mods,
      liveMod: liveMod,
      assigning: assigning,
      hostParam: hostFlash,
      onMenu: onMenu,
      onAssignTo: p => add(p, {
        slot: assigning.slot,
        name: assigning.name
      }),
      compact: compact,
      hoverId: hoverId,
      setHoverId: setHoverId
    }))), /*#__PURE__*/React.createElement(SamplerStatusBar, {
      assigning: assigning,
      onEndAssign: () => setAssigning(null),
      compact: compact,
      voices: `${voicesRef.current.length} active · limit ${pad ? pad.voices : 4} per sound`
    }), menuSpec && /*#__PURE__*/React.createElement("div", {
      onClick: e => e.stopPropagation()
    }, /*#__PURE__*/React.createElement(ContextMenu, {
      key: menu.param + menu.sub,
      x: menu.x,
      y: menu.y,
      style: {
        position: 'absolute'
      },
      title: menuSpec.title,
      items: menuSpec.items,
      onClose: () => setMenu(null)
    }))), /*#__PURE__*/React.createElement("div", {
      style: {
        width: W,
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        flexWrap: 'wrap',
        padding: '8px 10px',
        border: '1px dashed var(--dd-line-2)',
        borderRadius: 4,
        fontFamily: 'var(--font-ui)',
        fontSize: 12,
        color: 'var(--dd-paper-3)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        fontWeight: 700,
        letterSpacing: '0.08em',
        textTransform: 'uppercase',
        fontSize: 11
      }
    }, "Mockup controls"), /*#__PURE__*/React.createElement(Toggle, {
      compact: true,
      label: "Play demo pattern",
      checked: pattern,
      onChange: setPattern
    }), /*#__PURE__*/React.createElement("span", null, "Host automates"), /*#__PURE__*/React.createElement(SegmentedControl, {
      compact: true,
      value: hostParam || 'none',
      onChange: v => setHostParam(v === 'none' ? null : v),
      options: [{
        id: 'none',
        label: 'Off'
      }, {
        id: 'level',
        label: 'Level'
      }, {
        id: 'pan',
        label: 'Pan'
      }, {
        id: 'start',
        label: 'Start'
      }]
    }), /*#__PURE__*/React.createElement(Toggle, {
      compact: true,
      label: "Missing sample",
      checked: missing,
      onChange: setMissing
    }), /*#__PURE__*/React.createElement(SegmentedControl, {
      compact: true,
      value: patchKind,
      onChange: v => {
        setPatchKind(v);
        setView(v === 'break' ? 'slices' : 'region');
      },
      options: [{
        id: 'kit',
        label: 'Drum kit'
      }, {
        id: 'break',
        label: 'Break patch'
      }]
    }))));
  }
  Object.assign(window, {
    SamplerApp
  });
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/das-sampler/App.jsx", error: String((e && e.message) || e) }); }

// ui_kits/das-sampler/Header.jsx
try { (() => {
// Das Sampler — header bar and status bar.
(() => {
  const {
    Button,
    IconButton,
    SegmentedControl,
    Meter,
    StatusMessage,
    MenuButton,
    ModGlyph
  } = window.DandrumDesignSystem_3c2eab;
  function SamplerHeader({
    patch,
    status,
    compact,
    onLayout,
    onReload,
    onPatchMenu,
    levels
  }) {
    return /*#__PURE__*/React.createElement("header", {
      style: {
        height: compact ? 36 : 44,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        gap: compact ? 8 : 12,
        padding: compact ? '0 8px' : '0 12px',
        background: 'var(--dd-ink-0)',
        borderBottom: '1px solid var(--border-hairline)'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'baseline',
        gap: 8,
        whiteSpace: 'nowrap'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-display)',
        fontWeight: 700,
        fontSize: compact ? 15 : 'var(--type-brand)',
        letterSpacing: '-0.01em',
        color: 'var(--text-primary)'
      }
    }, "dandrum"), /*#__PURE__*/React.createElement("span", {
      style: {
        width: 6,
        height: 6,
        borderRadius: 6,
        background: 'var(--dd-vermilion)',
        alignSelf: 'center'
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-ui)',
        fontWeight: 700,
        fontSize: compact ? 'var(--type-label)' : 'var(--type-heading)',
        letterSpacing: 'var(--tracking-heading)',
        textTransform: 'uppercase',
        color: 'var(--text-secondary)'
      }
    }, "Das Sampler")), /*#__PURE__*/React.createElement("div", {
      style: {
        width: 1,
        height: 20,
        background: 'var(--border-hairline)'
      }
    }), /*#__PURE__*/React.createElement(MenuButton, {
      label: "Patch",
      value: patch.name,
      width: compact ? 180 : 240,
      onClick: onPatchMenu,
      compact: compact
    }), !compact && /*#__PURE__*/React.createElement(StatusMessage, {
      inline: true,
      kind: status.kind,
      title: status.title
    }, status.text), compact && /*#__PURE__*/React.createElement(StatusMessage, {
      inline: true,
      compact: true,
      kind: status.kind,
      title: status.short || status.title
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1
      }
    }), /*#__PURE__*/React.createElement(Button, {
      variant: status.kind === 'ok' ? 'secondary' : 'primary',
      icon: "reload",
      size: compact ? 'sm' : 'md',
      onClick: onReload
    }, "Reload Patch"), /*#__PURE__*/React.createElement(SegmentedControl, {
      compact: true,
      value: compact ? 'compact' : 'full',
      onChange: onLayout,
      options: [{
        id: 'full',
        label: 'Full',
        title: '1200 × 800'
      }, {
        id: 'compact',
        label: 'Compact',
        title: '820 × 560'
      }]
    }), /*#__PURE__*/React.createElement(Meter, {
      levels: levels,
      thickness: 4,
      length: compact ? 24 : 30
    }));
  }
  function SamplerStatusBar({
    assigning,
    onEndAssign,
    voices,
    compact,
    focusHint
  }) {
    return /*#__PURE__*/React.createElement("footer", {
      style: {
        height: compact ? 24 : 26,
        flex: 'none',
        display: 'flex',
        alignItems: 'center',
        gap: 12,
        padding: '0 12px',
        background: 'var(--dd-ink-0)',
        borderTop: '1px solid var(--border-hairline)',
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-label)',
        color: 'var(--text-tertiary)',
        whiteSpace: 'nowrap',
        overflow: 'hidden'
      }
    }, assigning ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        color: 'var(--text-primary)',
        fontWeight: 600
      }
    }, /*#__PURE__*/React.createElement(ModGlyph, {
      slot: assigning.slot,
      size: 9
    }), "Assigning ", assigning.name), /*#__PURE__*/React.createElement("span", null, "Click highlighted controls to add \xB7 Esc to finish"), /*#__PURE__*/React.createElement(Button, {
      size: "sm",
      variant: "ghost",
      onClick: onEndAssign,
      style: {
        height: 20
      }
    }, "Done")) : /*#__PURE__*/React.createElement("span", null, focusHint || 'Right-click a live control, or focus it and press M, to assign modulation'), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        fontFamily: 'var(--font-value)',
        fontSize: 'var(--type-micro)'
      }
    }, "Voices ", voices));
  }
  Object.assign(window, {
    SamplerHeader,
    SamplerStatusBar
  });
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/das-sampler/Header.jsx", error: String((e && e.message) || e) }); }

// ui_kits/das-sampler/LiveControls.jsx
try { (() => {
// Das Sampler — live control strip (public host parameters) + modulation context menu wiring.
(() => {
  const {
    Panel,
    Knob,
    Tooltip,
    ModIndicator,
    Icon
  } = window.DandrumDesignSystem_3c2eab;
  const D = window.SAMPLER_DATA;
  function LiveControls({
    values,
    onChange,
    mods,
    liveMod,
    assigning,
    hostParam,
    focusedId,
    onMenu,
    onAssignTo,
    compact,
    hoverId,
    setHoverId
  }) {
    return /*#__PURE__*/React.createElement(Panel, {
      title: "Live controls",
      compact: compact,
      style: {
        flex: 1
      },
      summary: D.liveParams.map(p => p.fmt(values[p.id]) + (p.unit ? ' ' + p.unit : '')).join(' · '),
      actions: !compact && /*#__PURE__*/React.createElement("span", {
        style: {
          display: 'flex',
          alignItems: 'center',
          gap: 5,
          fontFamily: 'var(--font-ui)',
          fontSize: 'var(--type-label)',
          color: 'var(--text-tertiary)'
        }
      }, /*#__PURE__*/React.createElement(Icon, {
        name: "host",
        size: 14
      }), "Patch-wide host parameters \xB7 automate in your DAW")
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: 'repeat(5, minmax(0, 1fr))',
        alignItems: 'start',
        height: '100%'
      }
    }, D.liveParams.map((p, i) => {
      const m = (mods[p.id] || []).map(a => ({
        ...a,
        live: liveMod[a.slot]
      }));
      const isAssigningHere = assigning && !(mods[p.id] || []).some(a => a.slot === assigning.slot);
      return /*#__PURE__*/React.createElement("div", {
        key: p.id,
        style: {
          position: 'relative',
          display: 'flex',
          flexDirection: 'column',
          alignItems: 'center',
          gap: 4,
          borderLeft: i ? '1px solid var(--border-hairline)' : 'none'
        },
        onMouseEnter: () => setHoverId(p.id),
        onMouseLeave: () => setHoverId(null),
        onClickCapture: e => {
          if (assigning && isAssigningHere) {
            e.stopPropagation();
            onAssignTo(p.id);
          }
        }
      }, /*#__PURE__*/React.createElement(Knob, {
        size: compact ? 'md' : 'lg',
        label: p.label,
        bipolar: p.bipolar,
        value: values[p.id],
        defaultValue: p.def,
        valueText: p.fmt(values[p.id]),
        unit: p.unit,
        modulations: m.map(a => ({
          ...a,
          source: a.source
        })),
        assigning: isAssigningHere,
        parseValue: p.parse,
        hostAutomated: hostParam === p.id,
        focused: focusedId === p.id,
        onChange: v => onChange(p.id, v),
        onContextMenu: e => onMenu(p.id, e)
      }), !compact && /*#__PURE__*/React.createElement("span", {
        style: {
          fontFamily: 'var(--font-ui)',
          fontSize: 'var(--type-micro)',
          color: 'var(--text-disabled)'
        }
      }, p.range));
    })));
  }
  function buildModMenu({
    param,
    value,
    mods,
    setDepth,
    remove,
    startAssign,
    reset,
    hostParam,
    close,
    sub,
    setSub
  }) {
    const p = D.liveParams.find(x => x.id === param);
    const list = mods[param] || [];
    const used = new Set(list.map(a => a.slot));
    if (sub) {
      const items = [{
        label: 'Back',
        icon: 'chevron-left',
        onSelect: () => setSub(false)
      }, {
        separator: true
      }, {
        header: 'Choose a source'
      }];
      D.modSources.forEach(s => items.push({
        label: s.name,
        shortcut: s.slot,
        disabled: used.has(s.slot),
        onSelect: () => startAssign(param, s)
      }));
      items.push({
        separator: true
      }, {
        note: 'After choosing, eligible controls are highlighted. Click more to add the same source; Esc finishes.'
      });
      return {
        title: `Assign modulation · ${p.label}`,
        items
      };
    }
    const items = [{
      header: 'In-plugin modulation'
    }];
    list.forEach(a => items.push({
      type: 'assignment',
      slot: a.slot,
      source: a.source,
      depth: a.depth,
      onDepth: d => setDepth(param, a.slot, d),
      onRemove: () => remove(param, a.slot)
    }));
    if (!list.length) items.push({
      note: 'No assignments yet.'
    });
    items.push({
      label: 'Assign modulation…',
      icon: 'modulate',
      submenu: true,
      disabled: used.size >= D.modSources.length,
      onSelect: () => setSub(true)
    });
    items.push({
      separator: true
    });
    items.push({
      label: 'Reset to default',
      icon: 'reset',
      shortcut: 'Dbl-click',
      onSelect: () => {
        reset(param);
        close();
      }
    });
    items.push({
      label: 'Type value…',
      shortcut: 'Enter',
      onSelect: close
    });
    if (list.length > 1) items.push({
      label: 'Remove all modulation',
      danger: true,
      onSelect: () => {
        list.forEach(a => remove(param, a.slot));
      }
    });
    items.push({
      separator: true
    });
    items.push({
      note: hostParam === param ? 'Your DAW is automating this parameter. Host automation and DAW modulation are edited in the DAW.' : 'Host automation and DAW-side modulation are edited in your DAW, not here.'
    });
    return {
      title: `${p.label} · ${p.fmt(value)}${p.unit ? ' ' + p.unit : ''}`,
      items
    };
  }
  Object.assign(window, {
    LiveControls,
    buildModMenu
  });
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/das-sampler/LiveControls.jsx", error: String((e && e.message) || e) }); }

// ui_kits/das-sampler/PadDetails.jsx
try { (() => {
// Das Sampler — selected pad details (prepared, read-only).
(() => {
  const {
    Panel,
    Rollout,
    SectionHeading,
    PropertyRow,
    ListRow,
    StatusMessage,
    Button,
    EmptyState
  } = window.DandrumDesignSystem_3c2eab;
  // noteName is lowercase, so the compiled bundle doesn't expose it on the namespace — keep a local copy (C4 = 60).
  const NOTE_NAMES = ['C', 'C♯', 'D', 'D♯', 'E', 'F', 'F♯', 'G', 'G♯', 'A', 'A♯', 'B'];
  const noteName = n => NOTE_NAMES[n % 12] + (Math.floor(n / 12) - 1);
  function PadDetails({
    note,
    pad,
    regionIdx,
    onRegion,
    activeRegion,
    compact,
    missingRegionId,
    patch
  }) {
    if (patch.kind === 'break') {
      const i = note - 36;
      const ok = i >= 0 && i < patch.slices.length;
      return /*#__PURE__*/React.createElement(Panel, {
        title: "Pad details",
        tag: "Prepared",
        prepared: true,
        compact: compact,
        style: {
          flex: 1
        }
      }, ok ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(PropertyRow, {
        label: "Note",
        value: `${note} · ${noteName(note)}`,
        prepared: true,
        compact: compact
      }), /*#__PURE__*/React.createElement(PropertyRow, {
        label: "Plays",
        value: `Slice ${i + 1}${patch.sliceNames ? ' · ' + patch.sliceNames[i] : ''}`,
        prepared: true,
        compact: compact
      }), /*#__PURE__*/React.createElement(PropertyRow, {
        label: "Sample",
        value: patch.file,
        prepared: true,
        compact: compact
      }), /*#__PURE__*/React.createElement(PropertyRow, {
        label: "Voice limit",
        value: "4",
        prepared: true,
        compact: compact
      }), /*#__PURE__*/React.createElement(PropertyRow, {
        label: "Steal",
        value: "Oldest",
        prepared: true,
        compact: compact
      }), /*#__PURE__*/React.createElement(PropertyRow, {
        label: "Choke",
        value: "Slices cut each other",
        prepared: true,
        compact: compact
      })) : /*#__PURE__*/React.createElement(EmptyState, {
        compact: true,
        title: "No slice on this note"
      }, "Notes 36\u201343 play slices 1\u20138."));
    }
    if (!pad) {
      return /*#__PURE__*/React.createElement(Panel, {
        title: "Pad details",
        tag: "Prepared",
        prepared: true,
        compact: compact,
        style: {
          flex: 1
        }
      }, /*#__PURE__*/React.createElement(PropertyRow, {
        label: "Note",
        value: `${note} · ${noteName(note)}`,
        prepared: true,
        compact: compact
      }), /*#__PURE__*/React.createElement("div", {
        style: {
          marginTop: 12
        }
      }, /*#__PURE__*/React.createElement(EmptyState, {
        compact: true,
        icon: "plus",
        title: "Empty pad"
      }, "Available for a future mapping in the patch.")));
    }
    const regions = pad.layers.flatMap(l => l.regions.map(r => ({
      ...r,
      layerName: l.name,
      vel: l.vel,
      policy: l.policy
    })));
    const r = regions[regionIdx] || regions[0];
    const altPolicy = pad.layers.some(l => l.policy) ? pad.layers.find(l => l.policy).policy : 'None';
    const g2 = {
      display: 'grid',
      gridTemplateColumns: compact ? 'minmax(0,1fr)' : 'minmax(0,1fr) minmax(0,1fr)',
      columnGap: 12
    };
    return /*#__PURE__*/React.createElement(Panel, {
      title: "Pad details",
      tag: "Prepared",
      prepared: true,
      compact: compact,
      style: {
        flex: 1
      },
      summary: `${pad.name} · ${note}`,
      bodyStyle: {
        display: 'flex',
        flexDirection: 'column',
        gap: 2,
        overflowY: 'auto',
        paddingTop: compact ? 4 : 6
      }
    }, /*#__PURE__*/React.createElement(Rollout, {
      title: "Mapping",
      prepared: true,
      compact: compact,
      summary: `${pad.name} · ${note} ${noteName(note)} · ${pad.mode}`
    }, /*#__PURE__*/React.createElement("div", {
      style: g2
    }, /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Sound",
      value: pad.name,
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Key",
      value: `${note} · ${noteName(note)}`,
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Velocity",
      value: pad.layers.length > 1 ? '2 layers' : '1–127',
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Play mode",
      value: pad.mode,
      prepared: true,
      compact: true
    }))), /*#__PURE__*/React.createElement(Rollout, {
      title: "Voices & choke",
      prepared: true,
      compact: compact,
      defaultCollapsed: compact,
      summary: `${pad.voices} voices · ${pad.steal} · ${pad.choke ? 'choke ' + pad.choke : 'no choke'}`
    }, /*#__PURE__*/React.createElement("div", {
      style: g2
    }, /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Voice limit",
      value: String(pad.voices),
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Steal",
      value: pad.steal,
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Choke",
      value: pad.choke ? `Group ${pad.choke}` : 'None',
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Alternates",
      value: altPolicy,
      prepared: true,
      compact: true
    }))), /*#__PURE__*/React.createElement(Rollout, {
      title: "Layers & alternates",
      prepared: true,
      compact: compact,
      defaultCollapsed: compact,
      summary: `${regions.length} region${regions.length > 1 ? 's' : ''}${pad.layers.length > 1 ? ' · ' + pad.layers.length + ' layers' : ''} · ${altPolicy}`
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 1,
        marginLeft: -10
      }
    }, regions.map((x, i) => /*#__PURE__*/React.createElement(ListRow, {
      key: x.id,
      compact: true,
      selected: i === regionIdx,
      active: x.id === activeRegion,
      onClick: () => onRegion(i),
      label: x.layerName + (x.alt ? ` · alt ${x.alt}` : ''),
      detail: pad.layers.length > 1 ? `vel ${x.vel[0]}–${x.vel[1]}` : null,
      value: x.id === missingRegionId ? 'missing' : x.weight ? `w ${x.weight}` : x.alt ? 'RR' : null,
      disabled: x.id === missingRegionId
    })))), r && /*#__PURE__*/React.createElement(Rollout, {
      title: 'Region · ' + r.file,
      prepared: true,
      compact: compact,
      defaultCollapsed: compact,
      summary: `${r.gain} dB · ${r.pan} · ${r.pitch}×`
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'grid',
        gridTemplateColumns: compact ? 'minmax(0,1fr)' : 'repeat(3, minmax(0,1fr))',
        columnGap: 12
      }
    }, /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Gain",
      value: r.gain,
      unit: "dB",
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Pan",
      value: r.pan,
      prepared: true,
      compact: true
    }), /*#__PURE__*/React.createElement(PropertyRow, {
      label: "Pitch",
      value: r.pitch,
      unit: "\xD7",
      prepared: true,
      compact: true
    }))));
  }
  Object.assign(window, {
    PadDetails
  });
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/das-sampler/PadDetails.jsx", error: String((e && e.message) || e) }); }

// ui_kits/das-sampler/PadGrid.jsx
try { (() => {
// Das Sampler — 4×4 pad grid with choke bracket.
(() => {
  const {
    PadCell,
    Panel,
    Icon: PadIcon
  } = window.DandrumDesignSystem_3c2eab;
  const PAD_ROWS = [[48, 49, 50, 51], [44, 45, 46, 47], [40, 41, 42, 43], [36, 37, 38, 39]];
  function PadGrid({
    patch,
    selected,
    onSelect,
    onTrigger,
    activity,
    compact,
    chokeFlash,
    missing
  }) {
    const size = compact ? 52 : 76,
      gap = compact ? 5 : 6;
    const pads = patch.kind === 'kit' ? patch.pads : null;
    const info = n => {
      if (pads) return pads[n];
      const i = n - 36;
      return i >= 0 && i < patch.slices.length ? {
        name: patch.sliceNames && patch.sliceNames[i] || 'Slice ' + (i + 1),
        short: patch.sliceNames && patch.sliceNames[i] || 'Slice ' + (i + 1),
        layers: [{
          regions: [{}]
        }]
      } : null;
    };
    // choke bracket geometry: column 3 (index 2), rows of 46 (1) and 42 (2)
    const hasChoke = pads && pads[42] && pads[46];
    const bx = 3 * size + 2 * gap + gap / 2;
    const by0 = 1 * (size + gap) + size * 0.3,
      by1 = 2 * (size + gap) + size * 0.7;
    return /*#__PURE__*/React.createElement(Panel, {
      title: "Pads",
      tag: patch.kind === 'kit' ? 'Notes 36–51' : 'Slices → 36–43',
      compact: compact,
      style: {
        flex: 'none'
      },
      summary: `selected ${selected}`
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        position: 'relative',
        display: 'grid',
        gridTemplateColumns: `repeat(4, ${size}px)`,
        gap
      }
    }, PAD_ROWS.flat().map(n => {
      const p = info(n);
      const a = activity[n] || {};
      if (!p) return /*#__PURE__*/React.createElement(PadCell, {
        key: n,
        size: size,
        note: n,
        mapped: false,
        selected: selected === n,
        compact: compact,
        onSelect: onSelect
      });
      const layer = p.layers.length > 1 ? p.layers.length : 0;
      const alts = Math.max(...p.layers.map(l => l.regions.length));
      return /*#__PURE__*/React.createElement(PadCell, {
        key: n,
        size: size,
        note: n,
        name: compact ? p.short : p.name,
        compact: compact,
        selected: selected === n,
        level: a.level || 0,
        velocity: a.velocity || 0,
        layers: layer,
        activeLayer: a.layer ?? -1,
        alternates: alts > 1 ? alts : 0,
        activeAlternate: a.alt ?? -1,
        chokeGroup: p.choke ?? undefined,
        choked: a.choked,
        missing: missing && n === 46,
        onSelect: onSelect,
        onTrigger: onTrigger
      });
    }), hasChoke && /*#__PURE__*/React.createElement("div", {
      "aria-hidden": "true",
      style: {
        position: 'absolute',
        left: bx - 1,
        top: by0,
        width: 2,
        height: by1 - by0,
        background: chokeFlash ? 'var(--dd-paper-1)' : 'var(--dd-line-3)',
        borderRadius: 1,
        transition: 'background 120ms'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: -3,
        top: -1,
        width: 8,
        height: 2,
        background: 'inherit'
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        position: 'absolute',
        left: -3,
        bottom: -1,
        width: 8,
        height: 2,
        background: 'inherit'
      }
    }))), hasChoke && !compact && /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 6,
        marginTop: 8,
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-label)',
        color: 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 2,
        padding: '1px 4px',
        borderRadius: 2,
        background: 'var(--dd-ink-0)',
        color: 'var(--text-secondary)',
        fontWeight: 600
      }
    }, /*#__PURE__*/React.createElement(PadIcon, {
      name: "choke",
      size: 12
    }), "1"), "Choke group 1: Closed Hat and Open Hat cut each other"));
  }
  Object.assign(window, {
    PadGrid,
    PAD_ROWS
  });
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/das-sampler/PadGrid.jsx", error: String((e && e.message) || e) }); }

// ui_kits/das-sampler/WaveView.jsx
try { (() => {
// Das Sampler — waveform panel with Region / Slices views.
(() => {
  const {
    Panel,
    Tabs,
    WaveformPanel,
    EmptyState,
    NumericField,
    ListRow,
    Button,
    Icon,
    SegmentedControl
  } = window.DandrumDesignSystem_3c2eab;
  const tagStyle = {
    display: 'inline-flex',
    alignItems: 'center',
    gap: 4,
    height: 20,
    padding: '0 6px',
    borderRadius: 2,
    border: '1px solid var(--border-control)',
    fontFamily: 'var(--font-ui)',
    fontSize: 'var(--type-micro)',
    fontWeight: 600,
    letterSpacing: 'var(--tracking-caps)',
    textTransform: 'uppercase',
    color: 'var(--text-secondary)',
    whiteSpace: 'nowrap'
  };
  function WaveView({
    patch,
    view,
    onView,
    pad,
    region,
    regionIdx,
    onRegion,
    cursor,
    startOffset,
    hostStart,
    startMod,
    compact,
    height,
    slice,
    onSlice,
    onLoadBreak,
    onLoadKit,
    missingRegion
  }) {
    const isBreak = patch.kind === 'break';
    const allRegions = pad ? pad.layers.flatMap(l => l.regions.map(r => ({
      ...r,
      layer: l.name,
      multi: pad.layers.length > 1
    }))) : [];
    const tabs = [{
      id: 'region',
      label: 'Region'
    }, {
      id: 'slices',
      label: 'Slices',
      badge: 'Break patch'
    }];
    const h = height ?? (compact ? 150 : 250);
    return /*#__PURE__*/React.createElement(Panel, {
      compact: compact,
      padding: 0,
      style: {
        flex: 'none'
      },
      summary: view === 'slices' ? 'Slices' : region ? region.file : 'Region',
      title: /*#__PURE__*/React.createElement("div", {
        "data-panel-action": "",
        style: {
          marginLeft: compact ? -6 : -8
        }
      }, /*#__PURE__*/React.createElement(Tabs, {
        compact: compact,
        value: view,
        onChange: onView,
        items: tabs
      })),
      actions: view === 'region' && pad && !isBreak ? /*#__PURE__*/React.createElement("div", {
        style: {
          display: 'flex',
          alignItems: 'center',
          gap: 6
        }
      }, !compact && /*#__PURE__*/React.createElement("span", {
        style: tagStyle
      }, /*#__PURE__*/React.createElement(Icon, {
        name: pad.mode === 'Gated' ? 'gate' : 'one-shot',
        size: 12
      }), pad.mode), allRegions.length > 1 && /*#__PURE__*/React.createElement(SegmentedControl, {
        compact: true,
        value: String(regionIdx),
        onChange: v => onRegion(Number(v)),
        options: allRegions.map((r, i) => ({
          id: String(i),
          label: r.multi ? r.layer + (r.alt ? ' ' + r.alt : '') : 'Alt ' + r.alt
        }))
      })) : null
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        padding: compact ? 8 : 12,
        display: 'flex',
        flexDirection: 'column',
        gap: 8
      }
    }, view === 'region' && !isBreak && (pad && region ? /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(WaveformPanel, {
      kind: region.kind,
      height: h,
      regionStart: region.start,
      regionEnd: region.end,
      fadeIn: region.fadeIn / region.len,
      fadeOut: region.fadeOut / region.len,
      cursor: cursor,
      startOffset: startOffset,
      hostAutomated: hostStart,
      startModulation: startMod,
      compact: compact,
      missing: missingRegion,
      label: `${region.file} · ${region.len} ms`
    }), !compact && /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        gap: 16,
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-label)',
        color: 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement("span", null, "Region ", /*#__PURE__*/React.createElement("b", {
      style: {
        color: 'var(--text-secondary)',
        fontFamily: 'var(--font-value)',
        fontWeight: 500
      }
    }, Math.round(region.start * region.len), "\u2013", Math.round(region.end * region.len), " ms")), /*#__PURE__*/React.createElement("span", null, "Fade in ", /*#__PURE__*/React.createElement("b", {
      style: {
        color: 'var(--text-secondary)',
        fontFamily: 'var(--font-value)',
        fontWeight: 500
      }
    }, region.fadeIn, " ms")), /*#__PURE__*/React.createElement("span", null, "Fade out ", /*#__PURE__*/React.createElement("b", {
      style: {
        color: 'var(--text-secondary)',
        fontFamily: 'var(--font-value)',
        fontWeight: 500
      }
    }, region.fadeOut, " ms")), /*#__PURE__*/React.createElement("span", null, "Loop ", /*#__PURE__*/React.createElement("b", {
      style: {
        color: 'var(--text-secondary)',
        fontWeight: 500
      }
    }, "none")), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1
      }
    }), /*#__PURE__*/React.createElement("span", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 4
      }
    }, /*#__PURE__*/React.createElement(Icon, {
      name: "lock",
      size: 12
    }), "Markers show prepared data"))) : /*#__PURE__*/React.createElement("div", {
      style: {
        height: h + (compact ? 0 : 24),
        display: 'flex'
      }
    }, /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center'
      }
    }, /*#__PURE__*/React.createElement(EmptyState, {
      icon: "plus",
      title: "No sound on this pad",
      compact: compact
    }, "This note is available for a future mapping. Add it to the patch, then reload.")))), view === 'region' && isBreak && /*#__PURE__*/React.createElement("div", {
      style: {
        height: h + 24,
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center'
      }
    }, /*#__PURE__*/React.createElement(EmptyState, {
      icon: "slice",
      title: "This patch plays slices",
      compact: compact,
      action: /*#__PURE__*/React.createElement(Button, {
        size: "sm",
        onClick: () => onView('slices')
      }, "Open Slices")
    }, "Example Break is one prepared sample divided into 8 slices.")), view === 'slices' && !isBreak && /*#__PURE__*/React.createElement("div", {
      style: {
        height: h + (compact ? 0 : 24),
        display: 'flex',
        flexDirection: 'column',
        justifyContent: 'center'
      }
    }, /*#__PURE__*/React.createElement(EmptyState, {
      icon: "slice",
      title: "Slices belong to sliced-break patches",
      compact: compact,
      action: /*#__PURE__*/React.createElement(Button, {
        size: "sm",
        icon: "folder",
        onClick: onLoadBreak
      }, "Load example break patch")
    }, "Reference Drum Kit has no break sample. Its pads play separate sample regions.")), view === 'slices' && isBreak && /*#__PURE__*/React.createElement(React.Fragment, null, /*#__PURE__*/React.createElement(WaveformPanel, {
      kind: "break",
      height: h,
      slices: patch.slices.map((pos, i) => ({
        pos,
        name: patch.sliceNames && patch.sliceNames[i]
      })),
      selectedSlice: slice,
      cursor: cursor,
      compact: compact,
      label: `${patch.file} · ${patch.len} ms`
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        alignItems: 'center',
        gap: 12
      }
    }, /*#__PURE__*/React.createElement(NumericField, {
      label: "Slice index",
      value: slice + 1,
      min: 1,
      max: patch.slices.length,
      onChange: v => onSlice(v - 1),
      compact: compact
    }), /*#__PURE__*/React.createElement("div", {
      style: {
        display: 'flex',
        flexDirection: 'column',
        gap: 3,
        fontFamily: 'var(--font-ui)',
        fontSize: 'var(--type-label)',
        color: 'var(--text-tertiary)'
      }
    }, /*#__PURE__*/React.createElement("span", null, /*#__PURE__*/React.createElement("b", {
      style: {
        color: 'var(--text-primary)',
        fontWeight: 600
      }
    }, patch.sliceNames ? patch.sliceNames[slice] : ''), " \xB7 slice ", slice + 1, " of ", patch.slices.length, " \xB7 note ", /*#__PURE__*/React.createElement("b", {
      style: {
        fontFamily: 'var(--font-value)',
        color: 'var(--text-secondary)',
        fontWeight: 500
      }
    }, 36 + slice), " \xB7 ", Math.round(patch.slices[slice] * patch.len), "\u2013", Math.round((patch.slices[slice + 1] ?? 1) * patch.len), " ms"), /*#__PURE__*/React.createElement("span", null, "Slice index is a host parameter; mapped notes 36\u201343 also trigger slices.")), /*#__PURE__*/React.createElement("div", {
      style: {
        flex: 1
      }
    }), !compact && /*#__PURE__*/React.createElement(Button, {
      size: "sm",
      variant: "ghost",
      onClick: onLoadKit
    }, "Back to Reference Drum Kit")))));
  }
  Object.assign(window, {
    WaveView
  });
})();
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/das-sampler/WaveView.jsx", error: String((e && e.message) || e) }); }

// ui_kits/das-sampler/data.js
try { (() => {
// Mock patch data for the Das Sampler UI kit. Musician-facing names only.
window.SAMPLER_DATA = {
  kit: {
    name: 'Reference Drum Kit',
    kind: 'kit',
    regions: 7,
    pads: {
      36: {
        name: 'Kick',
        short: 'Kick',
        mode: 'One-shot',
        voices: 8,
        steal: 'Oldest',
        choke: null,
        layers: [{
          name: 'Kick',
          vel: [1, 127],
          policy: null,
          regions: [{
            id: 'kick',
            file: 'kick.wav',
            kind: 'kick',
            len: 410,
            gain: '0.0',
            pan: 'C',
            pitch: '1.000',
            fadeIn: 0,
            fadeOut: 30,
            start: 0.01,
            end: 0.92
          }]
        }]
      },
      38: {
        name: 'Snare',
        short: 'Snare',
        mode: 'One-shot',
        voices: 8,
        steal: 'Oldest',
        choke: null,
        layers: [{
          name: 'Soft',
          vel: [1, 95],
          policy: null,
          regions: [{
            id: 'snare-soft',
            file: 'snare_soft.wav',
            kind: 'snare-soft',
            len: 280,
            gain: '−2.0',
            pan: 'C',
            pitch: '1.000',
            fadeIn: 0,
            fadeOut: 25,
            start: 0.015,
            end: 0.85
          }]
        }, {
          name: 'Hard',
          vel: [96, 127],
          policy: 'Round-robin',
          regions: [{
            id: 'snare-hard-a',
            alt: 'A',
            file: 'snare_hard_a.wav',
            kind: 'snare',
            len: 320,
            gain: '0.0',
            pan: 'C',
            pitch: '1.000',
            fadeIn: 0,
            fadeOut: 30,
            start: 0.01,
            end: 0.88
          }, {
            id: 'snare-hard-b',
            alt: 'B',
            file: 'snare_hard_b.wav',
            kind: 'snare',
            len: 335,
            gain: '−0.5',
            pan: 'C',
            pitch: '0.995',
            fadeIn: 0,
            fadeOut: 30,
            start: 0.012,
            end: 0.9
          }]
        }]
      },
      42: {
        name: 'Closed Hat',
        short: 'Cl Hat',
        mode: 'One-shot',
        voices: 8,
        steal: 'Oldest',
        choke: 1,
        layers: [{
          name: 'Closed Hat',
          vel: [1, 127],
          policy: null,
          regions: [{
            id: 'hat-closed',
            file: 'hat_closed.wav',
            kind: 'hat-closed',
            len: 120,
            gain: '−3.0',
            pan: 'R8',
            pitch: '1.000',
            fadeIn: 0,
            fadeOut: 15,
            start: 0.005,
            end: 0.6
          }]
        }]
      },
      46: {
        name: 'Open Hat',
        short: 'Op Hat',
        mode: 'Gated',
        voices: 8,
        steal: 'Oldest',
        choke: 1,
        layers: [{
          name: 'Open Hat',
          vel: [1, 127],
          policy: 'Weighted',
          regions: [{
            id: 'hat-open-a',
            alt: 'A',
            weight: '1.0',
            file: 'hat_open_a.wav',
            kind: 'hat-open',
            len: 620,
            gain: '−3.5',
            pan: 'R8',
            pitch: '1.000',
            fadeIn: 0,
            fadeOut: 120,
            start: 0.01,
            end: 0.9
          }, {
            id: 'hat-open-b',
            alt: 'B',
            weight: '0.6',
            file: 'hat_open_b.wav',
            kind: 'hat-open',
            len: 590,
            gain: '−4.0',
            pan: 'R8',
            pitch: '1.000',
            fadeIn: 0,
            fadeOut: 120,
            start: 0.01,
            end: 0.88
          }]
        }]
      }
    }
  },
  breakPatch: {
    name: 'Example Break (sliced)',
    kind: 'break',
    file: 'break_120bpm.wav',
    len: 2000,
    slices: [0, 0.125, 0.25, 0.3125, 0.5, 0.625, 0.75, 0.875],
    sliceNames: ['Kick', 'Hat', 'Snare', 'Ghost snare', 'Kick', 'Hat open', 'Snare', 'Fill']
  },
  modSources: [{
    slot: 'A',
    name: 'Velocity'
  }, {
    slot: 'B',
    name: 'Random per hit'
  }, {
    slot: 'C',
    name: 'Mod wheel'
  }, {
    slot: 'D',
    name: 'Aftertouch'
  }],
  liveParams: [{
    id: 'pitch',
    label: 'Pitch ratio',
    bipolar: true,
    def: 0.5,
    fmt: v => Math.pow(2, (v - 0.5) * 2).toFixed(3),
    unit: '×',
    range: '0.500–2.000×',
    parse: t => {
      const r = parseFloat(t);
      return r > 0 ? Math.log2(r) / 2 + 0.5 : null;
    }
  }, {
    id: 'start',
    label: 'Start offset',
    bipolar: false,
    def: 0,
    fmt: v => String(Math.round(v * 100)),
    unit: 'ms',
    range: '0–100 ms',
    parse: t => {
      const n = parseFloat(t);
      return Number.isFinite(n) ? n / 100 : null;
    }
  }, {
    id: 'level',
    label: 'Level',
    bipolar: false,
    def: 0.8,
    fmt: v => v < 0.01 ? '−∞' : ((v - 0.8) * 30).toFixed(1).replace('-', '−'),
    unit: 'dB',
    range: '−∞…+6 dB',
    parse: t => {
      const n = parseFloat(String(t).replace('−', '-'));
      return Number.isFinite(n) ? n / 30 + 0.8 : null;
    }
  }, {
    id: 'pan',
    label: 'Pan',
    bipolar: true,
    def: 0.5,
    fmt: v => {
      const p = Math.round((v - 0.5) * 100);
      return p === 0 ? 'C' : p < 0 ? 'L' + -p : 'R' + p;
    },
    unit: '',
    range: 'L50…R50',
    parse: t => {
      const s = String(t).trim().toUpperCase();
      if (s === 'C') return 0.5;
      const n = parseFloat(s.slice(1));
      if (!Number.isFinite(n)) return null;
      return 0.5 + (s[0] === 'L' ? -n : n) / 100;
    }
  }, {
    id: 'variation',
    label: 'Variation',
    bipolar: false,
    def: 0,
    fmt: v => String(Math.round(v * 100)),
    unit: '%',
    range: 'alternate spread',
    parse: t => {
      const n = parseFloat(t);
      return Number.isFinite(n) ? n / 100 : null;
    }
  }]
};
})(); } catch (e) { __ds_ns.__errors.push({ path: "ui_kits/das-sampler/data.js", error: String((e && e.message) || e) }); }

__ds_ns.Button = __ds_scope.Button;

__ds_ns.IconButton = __ds_scope.IconButton;

__ds_ns.Knob = __ds_scope.Knob;

__ds_ns.NumericField = __ds_scope.NumericField;

__ds_ns.Slider = __ds_scope.Slider;

__ds_ns.Toggle = __ds_scope.Toggle;

__ds_ns.Switch = __ds_scope.Switch;

__ds_ns.KeyMap = __ds_scope.KeyMap;

__ds_ns.LayerStack = __ds_scope.LayerStack;

__ds_ns.ListRow = __ds_scope.ListRow;

__ds_ns.PropertyRow = __ds_scope.PropertyRow;

__ds_ns.ParamLabel = __ds_scope.ParamLabel;

__ds_ns.Meter = __ds_scope.Meter;

__ds_ns.MOD_SLOTS = __ds_scope.MOD_SLOTS;

__ds_ns.ModGlyph = __ds_scope.ModGlyph;

__ds_ns.ModIndicator = __ds_scope.ModIndicator;

__ds_ns.OutputBusses = __ds_scope.OutputBusses;

__ds_ns.PadCell = __ds_scope.PadCell;

__ds_ns.WaveformPanel = __ds_scope.WaveformPanel;

__ds_ns.StatusMessage = __ds_scope.StatusMessage;

__ds_ns.Tooltip = __ds_scope.Tooltip;

__ds_ns.EmptyState = __ds_scope.EmptyState;

__ds_ns.ICON_NAMES = __ds_scope.ICON_NAMES;

__ds_ns.Icon = __ds_scope.Icon;

__ds_ns.Panel = __ds_scope.Panel;

__ds_ns.Rollout = __ds_scope.Rollout;

__ds_ns.SectionHeading = __ds_scope.SectionHeading;

__ds_ns.ContextMenu = __ds_scope.ContextMenu;

__ds_ns.MenuButton = __ds_scope.MenuButton;

__ds_ns.Tabs = __ds_scope.Tabs;

__ds_ns.SegmentedControl = __ds_scope.SegmentedControl;

})();
