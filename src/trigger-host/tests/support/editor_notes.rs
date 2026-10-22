use super::{
    PreparedRuntime, RuntimeAudio, RuntimeControl, TriggerHostConfiguration, runtime_handoff,
};
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{HostBuses, PreparationContext};
use dandrum_instrument_session::InstrumentSession;

pub(crate) const FRAMES: usize = 16;
pub(crate) const PREFIX_L: [f32; 4] = [8191.0 / 32768.0, -0.5, 24575.0 / 32768.0, -0.25];
pub(crate) const PREFIX_R: [f32; 4] = [8191.0 / 65536.0, -0.25, 24575.0 / 65536.0, -0.125];
pub(crate) const TAIL: (f32, f32) = (-0.125, -0.0625);

pub(crate) struct Fixture {
    _directory: tempfile::TempDir,
    pub(crate) session: InstrumentSession,
}

impl Fixture {
    pub(crate) fn new(mode: &str, voices: usize) -> Self {
        Self::with_pan(mode, voices, -0.5)
    }

    pub(crate) fn with_pan(mode: &str, voices: usize, pan: f32) -> Self {
        Self::with_voice_velocity(mode, voices, pan, false)
    }

    pub(crate) fn with_voice_velocity(
        mode: &str,
        voices: usize,
        pan: f32,
        velocity_level: bool,
    ) -> Self {
        let directory = tempfile::tempdir().unwrap();
        let mut left = vec![TAIL.0; 16_384];
        let mut right = vec![TAIL.0; 16_384];
        left[..4].copy_from_slice(&[0.25, -0.5, 0.75, -0.25]);
        right[..4].copy_from_slice(&[0.25, -0.5, 0.75, -0.25]);
        dandrum_engine::wav::write_wav_stereo_i16(
            std::fs::File::create(directory.path().join("hit.wav")).unwrap(),
            48_000,
            &left,
            &right,
        )
        .unwrap();
        // Trigger metadata alone does not scale sample_player output. This
        // optional real control cable makes the voice's velocity audible.
        let velocity_connection = if velocity_level {
            "      - { from: voice.velocity, to: player.level }"
        } else {
            ""
        };
        let yaml = format!(
            r#"
assets:
  sample_sources:
    - id: hit
      path: hit.wav
      regions: [{{ id: body, start_frame: 0, end_frame: 16384 }}]
ports:
  - {{ name: master, direction: output, signal: audio, channels: 2, maps_from: voices.audio }}
module_definitions:
  - type: sample_voice
    ports:
      - {{ name: audio, direction: output, signal: audio, channels: 2, maps_from: player.audio }}
    modules:
      - {{ id: player, type: sample_player, static: {{ source: hit, region: body, channels: 2, mode: {mode} }}, defaults: {{ pan: {pan} }} }}
    connections:
      - {{ from: voice.gate, to: player.trigger }}
      - {{ from: voice.gate, to: player.gate }}
{velocity_connection}
modules:
  - {{ id: midi, type: midi_input }}
  - {{ id: voices, type: poly, static: {{ definition: sample_voice, max_voices: {voices}, allocation: reject-new }} }}
connections:
  - {{ from: midi.events, to: voices.notes }}
"#
        );
        let session = InstrumentSession::new_for_planar_host(
            &yaml,
            PreparationContext::new(directory.path(), 48_000),
            RenderSettings {
                sample_rate_hz: 48_000,
                block_size_frames: FRAMES as u32,
                duration_frames: FRAMES as u64,
            },
            HostBuses::new().with_output("master", 2),
        )
        .unwrap();
        Self {
            _directory: directory,
            session,
        }
    }

    pub(crate) fn runtime(&self, capacity: usize) -> (RuntimeControl, RuntimeAudio) {
        runtime_handoff(self.prepared(), capacity).unwrap()
    }

    pub(crate) fn prepared(&self) -> PreparedRuntime {
        PreparedRuntime::new(
            self.session.prepared(),
            TriggerHostConfiguration {
                sample_rate_hz: dandrum_engine::render_settings::SampleRate::new(48_000.0).unwrap(),
                max_block_frames: FRAMES as u32,
            },
        )
        .unwrap()
    }
}
