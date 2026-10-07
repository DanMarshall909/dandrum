// The renderer receives a control only when its live value and prepared
// descriptor belong to the same instrument generation.
export function admittedParameter(state, document, id) {
  if (!Number.isInteger(state?.generation) || state.generation !== document?.generation)
    return null;
  const descriptor = document.parameters?.find(parameter => parameter.id === id);
  const live = state.parameters?.find(parameter => parameter.id === id);
  if (!descriptor || !live || actualValue(live.value, descriptor) === null) return null;
  return { ...descriptor, value: live.value, generation: state.generation, sequence: state.sequence };
}

function validRange(parameter) {
  return Number.isFinite(parameter?.minValue) && Number.isFinite(parameter?.maxValue)
    && parameter.maxValue > parameter.minValue;
}

export function actualValue(value, parameter) {
  if (!validRange(parameter) || !Number.isFinite(value) || value < 0 || value > 1) return null;
  return parameter.minValue + value * (parameter.maxValue - parameter.minValue);
}

export function formatActualValue(value, parameter) {
  const actual = actualValue(value, parameter);
  return actual === null ? '' : String(Number(actual.toPrecision(6)));
}

export function parseActualValue(text, parameter) {
  if (typeof text !== 'string' || !validRange(parameter)) return null;
  const decimal = text.trim().replaceAll('−', '-');
  if (!/^[+-]?(?:\d+(?:\.\d*)?|\.\d+)(?:e[+-]?\d+)?$/i.test(decimal)) return null;
  const actual = Number(decimal);
  if (!Number.isFinite(actual) || actual < parameter.minValue || actual > parameter.maxValue)
    return null;
  return (actual - parameter.minValue) / (parameter.maxValue - parameter.minValue);
}

const clamp = value => Math.max(0, Math.min(1, value));

export function dragValue(startValue, startY, y, fine) {
  return clamp(startValue + (startY - y) / (fine ? 800 : 200));
}

export function keyValue(value, key, fine, defaultValue) {
  const step = fine ? 0.01 : 0.05;
  if (key === 'ArrowUp' || key === 'ArrowRight') return clamp(value + step);
  if (key === 'ArrowDown' || key === 'ArrowLeft') return clamp(value - step);
  if (key === 'Home') return 0;
  if (key === 'End') return 1;
  if (key === 'Delete' || key === 'Backspace')
    return Number.isFinite(defaultValue) && defaultValue >= 0 && defaultValue <= 1
      ? defaultValue : null;
  return null;
}

export function wheelValue(value, deltaY, fine) {
  return deltaY === 0 ? null : clamp(value + (deltaY < 0 ? 1 : -1) * (fine ? 0.01 : 0.02));
}
