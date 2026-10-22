use dandrum_engine::PreparedSamplerAssets;
use dandrum_engine::graph_processor::RealtimeGraphProcessor;
use dandrum_engine::kernel::document::load_kernel_patch_str;
use dandrum_engine::patch::RenderSettings;
use dandrum_engine::preparation::{HostBuses, prepare_kernel_graph_with_buses};

const FRAMES: usize = 8;
const GAIN_FILTER_PATCH: &str = r#"
ports:
  - { name: input, direction: input, signal: audio, channels: 2, maps_to: amp.audio_in }
  - { name: master, direction: output, signal: audio, channels: 2, maps_from: tone.audio_out }
modules:
  - { id: amp, type: gain, static: { channels: 2 }, defaults: { gain: 0.5 } }
  - { id: tone, type: filter, static: { algorithm: biquad, mode: lowpass, channels: 2 }, defaults: { cutoff: 1.0, resonance: 0.09090909 } }
connections:
  - { from: amp.audio_out, to: tone.audio_in }
"#;

struct RenderFixture {
    runtime: RealtimeGraphProcessor,
    inputs: Vec<Vec<Vec<f32>>>,
    outputs: Vec<Vec<Vec<f32>>>,
}

impl RenderFixture {
    fn prepare() -> Self {
        let patch = load_kernel_patch_str(GAIN_FILTER_PATCH).expect("gain/filter patch loads");
        let settings = RenderSettings {
            sample_rate_hz: 48_000,
            block_size_frames: FRAMES as u32,
            duration_frames: FRAMES as u64,
        };
        let prepared = prepare_kernel_graph_with_buses(
            patch.root(),
            patch.registry(),
            &settings,
            &HostBuses::new()
                .with_input("input", 2)
                .with_output("master", 2),
        )
        .expect("stereo gain/filter graph prepares");
        let runtime = RealtimeGraphProcessor::from_compiled_patch(
            prepared.compiled_patch().clone(),
            settings.sample_rate_hz as f32,
            &PreparedSamplerAssets::empty(),
            FRAMES,
        );
        let mut inputs = vec![vec![vec![0.0; FRAMES]; 2]];
        inputs[0][0][0] = -0.5;
        inputs[0][1][0] = 0.25;
        Self {
            runtime,
            inputs,
            outputs: vec![vec![vec![0.0; FRAMES]; 2]],
        }
    }

    fn render(&mut self) {
        assert_eq!(
            self.runtime
                .render_root_buses(&self.inputs, &mut self.outputs),
            FRAMES
        );
    }

    fn assert_initial_impulse(&self) {
        // Independently derived RBJ low-pass impulse: cutoff=8 kHz, Q=1,
        // sample rate=48 kHz, preceding gain=0.5. Resonance's float control
        // approximation changes these values by less than the 1e-6 tolerance.
        let expected = [
            [-0.04361441, -0.11766428, -0.10846761, -0.029136838],
            [0.021807205, 0.05883214, 0.054233804, 0.014568419],
        ];
        for (channel, expected) in self.outputs[0].iter().zip(expected) {
            for (&actual, expected) in channel.iter().zip(expected) {
                assert!(
                    (actual - expected).abs() <= 1.0e-6,
                    "signed gain/filter sample: expected {expected}, got {actual}"
                );
            }
        }
    }
}

#[test]
fn gain_filter_signed_output_and_reset_on_control_thread() {
    let mut fixture = RenderFixture::prepare();
    fixture.render();
    fixture.assert_initial_impulse();
    fixture.runtime.reset();
    fixture.render();
    fixture.assert_initial_impulse();
}

fn render_on_another_thread(mut fixture: RenderFixture) -> RenderFixture {
    let control_thread = std::thread::current().id();
    std::thread::spawn(move || {
        assert_ne!(std::thread::current().id(), control_thread);
        fixture.render();
        fixture
    })
    .join()
    .expect("render thread returns exclusive runtime ownership")
}

#[test]
fn owned_runtime_transfers_to_render_thread_with_signed_output() {
    fn assert_send<T: Send>() {}
    assert_send::<RealtimeGraphProcessor>();

    let fixture = render_on_another_thread(RenderFixture::prepare());
    fixture.assert_initial_impulse();
    // The returned runtime and every prepared buffer are reclaimed here,
    // after joining, rather than on the render thread.
    drop(fixture);
}

#[test]
fn returned_runtime_resets_and_instances_keep_independent_filter_state() {
    let first = RenderFixture::prepare();
    let second = RenderFixture::prepare();

    let first = render_on_another_thread(first);
    first.assert_initial_impulse();
    let mut first = render_on_another_thread(first);
    assert!(
        (first.outputs[0][0][0] + 0.04361441).abs() > 1.0e-5,
        "the returned first instance must retain its filter tail"
    );

    let second = render_on_another_thread(second);
    second.assert_initial_impulse();

    first.runtime.reset();
    let first = render_on_another_thread(first);
    first.assert_initial_impulse();
    let second = render_on_another_thread(second);
    assert!(
        (second.outputs[0][0][0] + 0.04361441).abs() > 1.0e-5,
        "resetting the first instance must not reset the second's tail"
    );

    drop(first);
    drop(second);
}
