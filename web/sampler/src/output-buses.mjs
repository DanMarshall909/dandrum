export async function acknowledgeOutputClip(invoke, update, channel, generation, ticket) {
  if (await invoke('ackMeterClip', channel, generation, ticket) !== true)
    throw new Error('Output clip acknowledgement was rejected');
  update(current => acknowledgedOutputClip(current, channel, generation, ticket));
}

export function acknowledgedOutputClip(display, channel, generation, ticket) {
  if (display.generation !== generation || display.channels[channel]?.ticket !== ticket)
    return display;
  return { ...display, channels: display.channels.map((item, index) =>
    index === channel ? { ...item, clipped: false } : item) };
}

export function createOutputBuses(React) {
  const h = React.createElement;
  return function OutputBuses({ buses, generation, meter, onAcknowledge, headerIcon }) {
    return h('section', { className: 'panel dd-output-buses', 'aria-label': 'Output buses' },
      h('div', { className: 'section-heading' },
        h('h2', null, headerIcon, 'Output buses'), h('span', null, `${buses.length} buses`)),
      buses.length === 0 ? h('p', { role: 'status' }, 'Output bindings unavailable') :
      buses.map(bus => {
        const bound = bus.meterBusId === 'master' && bus.channels.length === 2;
        const measured = bound && meter.valid && meter.generation === generation;
        const status = !bound ? 'Measurements unavailable' : !measured ? 'WAITING FOR AUDIO'
          : meter.complete ? 'LIVE' : 'HISTORY GAP';
        return h('div', { key: bus.id, className: 'dd-output-bus', role: 'group', tabIndex: 0,
          'aria-label': `${bus.name} output bus`, 'data-output-bus': bus.id, 'data-generation': generation },
          h('div', { className: 'dd-bus-name' }, bus.main ? h('small', null, 'Main') : null,
            h('strong', null, bus.name)),
          h('div', { className: 'dd-bus-channels' }, bus.channels.length
            ? `${bus.channels.length} ${bus.channels.length === 1 ? 'channel' : 'channels'} · ${bus.channels.join('/')}`
            : 'Disabled'),
          h('div', { className: 'dd-bus-feeds' }, 'Feed details unavailable'),
          h('div', { className: 'dd-bus-measurements' }, h('span', { className: 'dd-bus-status' }, status),
            bus.channels.map((name, index) => {
              const channel = measured ? meter.channels[index] : null;
              return h('div', { key: index, className: 'dd-bus-channel', 'data-channel-index': index },
                h('strong', null, name),
                channel ? h('div', { className: 'dd-bus-meter', role: 'meter',
                  'aria-label': `${bus.name} ${name} peak level`, 'aria-valuemin': 0, 'aria-valuemax': 1,
                  'aria-valuenow': channel.peak, 'data-rms': channel.rms,
                  'data-meter-bus': bus.meterBusId, 'data-generation': meter.generation, 'data-channel-index': index },
                  h('i', { className: 'dd-bus-peak', style: { width: `${channel.peak * 100}%` } }),
                  h('i', { className: 'dd-bus-rms', style: { width: `${channel.rms * 100}%` } }))
                  : h('span', { className: 'dd-bus-missing' }, bound ? 'Waiting' : 'Unavailable'),
                bound ? h('button', { type: 'button', className: `dd-bus-clip${channel?.clipped ? ' latched' : ''}`,
                  'aria-label': `Acknowledge ${bus.name} ${name} clip`,
                  disabled: !onAcknowledge || !channel?.clipped || channel.ticket === '0',
                  onClick: () => onAcknowledge(bus.id, index, generation, channel.ticket) }, 'CLIP') : null);
            })));
      }));
  };
}
