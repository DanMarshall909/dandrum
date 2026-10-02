import { createParameterGesture } from '../sampler/src/parameter-gesture.mjs';
import { formatActualValue, parseActualValue, dragValue, keyValue, wheelValue } from './parameter-value.mjs';

// Adapted from the preserved design-system Knob.jsx. Each app supplies its own
// React instance; the shared component adds host gestures and prepared metadata.
// Modulation/automation provenance is unavailable, so no assignments are drawn.
export function createHostKnob(React) {
  const { useEffect, useMemo, useRef, useState } = React;
  const h = React.createElement;
  const point = (c, r, degrees) => {
    const angle = (degrees - 90) * Math.PI / 180;
    return [c + r * Math.cos(angle), c + r * Math.sin(angle)];
  };
  const arc = (c, r, from, to) => {
    if (Math.abs(to - from) < 0.01) return '';
    const a = Math.min(from, to), b = Math.max(from, to);
    const [x0, y0] = point(c, r, a), [x1, y1] = point(c, r, b);
    return `M${x0.toFixed(2)} ${y0.toFixed(2)}A${r} ${r} 0 ${b - a > 180 ? 1 : 0} 1 ${x1.toFixed(2)} ${y1.toFixed(2)}`;
  };

  return function HostKnob({ parameter, label, size = 48, command, onError }) {
    const disabled = parameter === null;
    const [local, setLocal] = useState(parameter?.value ?? 0);
    const [hovered, setHovered] = useState(false);
    const [popupHover, setPopupHover] = useState(false);
    const [focused, setFocused] = useState(false);
    const [dragging, setDragging] = useState(false);
    const [nudged, setNudged] = useState(false);
    const [editing, setEditing] = useState(false);
    const [draft, setDraft] = useState('');
    const root = useRef(null);
    const drag = useRef(null);
    const interaction = useRef(0);
    const edit = useRef(null);
    const closeTimer = useRef(null);
    const nudgeTimer = useRef(null);
    const authoritative = useRef(parameter);
    authoritative.current = parameter;
    const gesture = useMemo(() => createParameterGesture(
      (name, value, generation) => command(name, parameter.id,
        ...(value === undefined ? [] : [value]), generation),
      reason => {
        setLocal(authoritative.current?.value ?? 0);
        onError(reason);
      }), [parameter?.id, parameter?.generation]);

    useEffect(() => () => { void gesture.end(); }, [gesture]);
    useEffect(() => {
      ++interaction.current;
      drag.current = null;
      edit.current = null;
      setDragging(false);
      setEditing(false);
      setLocal(parameter?.value ?? 0);
    }, [parameter?.id, parameter?.generation]);
    useEffect(() => {
      if (!drag.current && !edit.current) setLocal(parameter?.value ?? 0);
    }, [parameter?.value]);
    useEffect(() => () => {
      clearTimeout(closeTimer.current);
      clearTimeout(nudgeTimer.current);
    }, []);

    const holdPopup = () => {
      clearTimeout(closeTimer.current);
      setPopupHover(true);
    };
    const leavePopup = () => {
      clearTimeout(closeTimer.current);
      closeTimer.current = setTimeout(() => setPopupHover(false), 250);
    };
    const nudge = () => {
      setNudged(true);
      clearTimeout(nudgeTimer.current);
      nudgeTimer.current = setTimeout(() => setNudged(false), 600);
    };
    const commit = next => {
      if (disabled || next === null) return;
      ++interaction.current;
      setLocal(next);
      nudge();
      gesture.commit(next, parameter.generation);
    };
    const finishDrag = () => {
      if (!drag.current) return;
      const releasedInteraction = interaction.current;
      drag.current = null;
      setDragging(false);
      void gesture.end().then(() => {
        // Host automation may have changed while local dragging hid its echo.
        // A later key, wheel, draft or drag owns the display once it starts.
        if (interaction.current === releasedInteraction && !drag.current && !edit.current)
          setLocal(authoritative.current?.value ?? 0);
      });
    };
    const startEdit = () => {
      if (disabled || edit.current) return;
      ++interaction.current;
      const initial = formatActualValue(local, parameter);
      edit.current = { parameter, initial };
      setDraft(initial);
      setEditing(true);
    };
    const finishEdit = (cancelled, returnFocus = false) => {
      const captured = edit.current;
      edit.current = null;
      setEditing(false);
      if (!captured) return;
      const matches = parameter && parameter.id === captured.parameter.id
        && parameter.generation === captured.parameter.generation;
      // Opening and closing a rounded readout must never rewrite the precise host value.
      const next = !cancelled && matches && draft !== captured.initial
        ? parseActualValue(draft, captured.parameter) : null;
      if (next !== null) commit(next);
      else setLocal(authoritative.current?.value ?? 0);
      if (returnFocus) root.current?.focus();
    };

    // React delegates wheel events passively. A local non-passive listener
    // keeps a value change from also scrolling the editor. Popup events belong
    // to the value editor and must never become a knob gesture.
    useEffect(() => {
      const element = root.current;
      const wheel = event => {
        if (disabled || element.querySelector('.dd-knob-popup')?.contains(event.target)) return;
        event.preventDefault();
        commit(wheelValue(local, event.deltaY, event.shiftKey));
      };
      element.addEventListener('wheel', wheel, { passive: false });
      return () => element.removeEventListener('wheel', wheel);
    }, [disabled, local, parameter?.generation, gesture]);

    const c = size / 2;
    const trackMax = size >= 60 ? 4 : size >= 44 ? 3 : 2.5;
    const trackMin = size >= 44 ? 1.5 : 1.25;
    const rTrack = c - (size >= 44 ? 6.5 : 5) - trackMax / 2;
    const rCap = rTrack - trackMax / 2 - (size >= 44 ? 2.5 : 2);
    const [x0, y0] = point(c, rTrack + trackMin / 2 + 1, -135);
    const [x1, y1] = point(c, rTrack + trackMin / 2 + (size >= 44 ? 4 : 3), -135);
    const active = !disabled && (dragging || nudged);
    const popup = !disabled && (popupHover || focused || dragging || nudged || editing);
    const title = label || parameter?.name || parameter?.id;
    const text = formatActualValue(local, parameter);
    const stop = event => event.stopPropagation();

    return h('div', {
      ref: root, className: 'dd-knob', role: 'slider', tabIndex: disabled ? -1 : 0,
      'aria-label': title, 'aria-valuemin': 0, 'aria-valuemax': 1,
      'aria-valuenow': disabled ? undefined : Number(local.toFixed(6)),
      'aria-valuetext': disabled ? undefined : text, 'aria-disabled': disabled,
      'data-parameter-id': parameter?.id, 'data-generation': parameter?.generation,
      'data-active': active, 'data-focused': focused, 'data-small': size <= 36,
      style: { minWidth: size + 12 },
      onPointerDown: event => {
        if (disabled || event.button !== 0 || drag.current) return;
        event.preventDefault();
        root.current.focus();
        event.currentTarget.setPointerCapture(event.pointerId);
        ++interaction.current;
        drag.current = { pointer: event.pointerId, y: event.clientY, value: local };
        setDragging(true);
        gesture.begin(parameter.generation);
      },
      onPointerMove: event => {
        const current = drag.current;
        if (!current || current.pointer !== event.pointerId) return;
        const next = dragValue(current.value, current.y, event.clientY, event.shiftKey);
        ++interaction.current;
        setLocal(next);
        gesture.change(next, parameter.generation);
      },
      onPointerUp: finishDrag, onPointerCancel: finishDrag, onLostPointerCapture: finishDrag,
      onDoubleClick: () => commit(keyValue(local, 'Delete', false, parameter?.normalisedDefaultValue)),
      onKeyDown: event => {
        if (disabled) return;
        if (event.key === 'Enter') { event.preventDefault(); startEdit(); return; }
        const next = keyValue(local, event.key, event.shiftKey, parameter.normalisedDefaultValue);
        if (next !== null) { event.preventDefault(); commit(next); }
      },
      onContextMenu: event => event.preventDefault(),
      onMouseEnter: () => { setHovered(true); holdPopup(); },
      onMouseLeave: () => { setHovered(false); leavePopup(); },
      onFocus: () => setFocused(true),
      onBlur: event => {
        if (!event.currentTarget.contains(event.relatedTarget)) {
          setFocused(false);
          finishDrag();
        }
      },
    },
    h('div', { className: 'dd-knob-label', title }, title),
    h('svg', { width: size, height: size, viewBox: `0 0 ${size} ${size}`, 'aria-hidden': true },
      h('path', { d: arc(c, rTrack, -135, 135), fill: 'none',
        stroke: disabled ? 'var(--dd-ink-4)' : 'var(--color-track)', strokeWidth: trackMin, strokeLinecap: 'round' }),
      !disabled && h('path', { 'data-dd-knob-part': 'value-arc', d: arc(c, rTrack, -135, -135 + local * 270),
        fill: 'none', stroke: 'var(--color-value)', strokeWidth: active ? trackMax : trackMin, strokeLinecap: 'round' }),
      h('line', { x1: x0, y1: y0, x2: x1, y2: y1, stroke: 'var(--dd-paper-3)',
        strokeWidth: 1.5, strokeLinecap: 'round', opacity: disabled ? 0.4 : 1 }),
      h('circle', { cx: c, cy: c + 1.5, r: rCap, fill: 'rgba(0,0,0,0.45)' }),
      h('circle', { 'data-dd-knob-part': 'cap', cx: c, cy: c, r: rCap,
        fill: disabled ? 'var(--dd-ink-4)' : dragging ? 'var(--dd-ink-6)' : hovered ? 'var(--dd-cap-hover)' : 'var(--dd-ink-5)',
        stroke: 'var(--dd-line-3)', strokeWidth: 1, strokeOpacity: disabled ? 0.3 : 0.6 }),
      h('path', { d: arc(c, rCap - 1, -60, 60), fill: 'none', stroke: 'rgba(255,255,255,0.09)', strokeWidth: 1 })),
    h('div', { className: 'dd-knob-warning-space', 'aria-hidden': true }),
    popup && h('div', { className: 'dd-knob-popup', role: 'dialog', 'aria-label': `${title} value`,
      onMouseEnter: holdPopup, onMouseLeave: leavePopup,
      onPointerDown: stop, onDoubleClick: stop, onWheel: stop, onKeyDown: stop },
      h('span', { className: 'dd-knob-popup-arrow', 'aria-hidden': true }),
      editing ? h('input', { className: 'dd-knob-input', type: 'text', inputMode: 'decimal',
        autoFocus: true, value: draft, 'aria-label': `${title} actual value`,
        onChange: event => setDraft(event.target.value), onFocus: event => event.target.select(),
        onBlur: () => finishEdit(false),
        onKeyDown: event => {
          event.stopPropagation();
          if (event.key === 'Enter' || event.key === 'Escape') {
            event.preventDefault(); finishEdit(event.key === 'Escape', true);
          }
        } })
        : h('button', { className: 'dd-knob-readout', type: 'button', 'data-dd-knob-value': true,
          'aria-label': `Edit ${title} actual value`, title: 'Click to type a value', onClick: startEdit }, text)));
  };
}
