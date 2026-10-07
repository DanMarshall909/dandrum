import { createKnobFace } from './knob-face.mjs';
import { acceptedKnobWrite, knobSnapshotValue, createKnobCommand } from './knob-reconciliation.mjs';
import { createParameterGesture } from '../sampler/src/parameter-gesture.mjs';
import { formatActualValue, parseActualValue, dragValue, keyValue, wheelValue } from './parameter-value.mjs';

// Adapted from the preserved design-system Knob.jsx. Each app supplies its own
// React instance; the shared component adds host gestures and prepared metadata.
// Modulation/automation provenance is unavailable, so no assignments are drawn.
export function createHostKnob(React) {
  const { useEffect, useMemo, useRef, useState } = React;
  const h = React.createElement;
  const KnobFace = createKnobFace(React);

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
    const lastWrite = useRef(null);
    const pendingCommit = useRef(null);
    const authoritative = useRef(parameter);
    authoritative.current = parameter;
    const gesture = useMemo(() => createParameterGesture(
      createKnobCommand((name, value, generation) => command(name, parameter.id,
        ...(value === undefined ? [] : [value]), generation), () => interaction.current,
      (reply, value) => { lastWrite.current = acceptedKnobWrite(lastWrite.current, reply, value); },
      owner => {
        if (owner === interaction.current && !pendingCommit.current)
          setLocal(knobSnapshotValue(authoritative.current, lastWrite.current));
      }), onError, () => interaction.current), [parameter?.id, parameter?.generation]);

    useEffect(() => () => { void gesture.end(); }, [gesture]);
    useEffect(() => {
      ++interaction.current;
      lastWrite.current = null;
      pendingCommit.current = null;
      drag.current = null;
      edit.current = null;
      setDragging(false);
      setEditing(false);
      setLocal(parameter?.value ?? 0);
    }, [parameter?.id, parameter?.generation]);
    useEffect(() => {
      if (!drag.current && !edit.current && !pendingCommit.current)
        setLocal(knobSnapshotValue(parameter, lastWrite.current));
    }, [parameter?.value, parameter?.sequence]);
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
      const owner = ++interaction.current;
      pendingCommit.current = owner;
      setLocal(next);
      nudge();
      gesture.commit(next, parameter.generation);
      void gesture.flush().then(() => {
        if (pendingCommit.current !== owner) return;
        pendingCommit.current = null;
        if (interaction.current === owner && !drag.current && !edit.current)
          setLocal(knobSnapshotValue(authoritative.current, lastWrite.current));
      });
    };
    const finishDrag = () => {
      if (!drag.current) return;
      const releasedInteraction = interaction.current;
      drag.current = null;
      setDragging(false);
      void gesture.end().then(() => {
        // A later interaction owns the display; otherwise retain the latest
        // accepted write until the host snapshot covers its admission sequence.
        if (interaction.current === releasedInteraction && !drag.current && !edit.current)
          setLocal(knobSnapshotValue(authoritative.current, lastWrite.current));
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
      else if (!pendingCommit.current) setLocal(knobSnapshotValue(authoritative.current, lastWrite.current));
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
    h(KnobFace, { size, value: local, disabled, active, dragging, hovered }),
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
