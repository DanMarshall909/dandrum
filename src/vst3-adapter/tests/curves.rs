mod common;
use common::{audio::*, parameters::*, *};
use vst3::Steinberg::{Vst::*, *};
#[test]
fn standard_parameter_queues_render_every_signed_ramp_and_jump_sample() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    start(&p, &a, 48000.0, 16);
    let declared = [
        (0x1001, 1.0 / 9.0),
        (0x1002, 0.0),
        (0x1003, 0.25),
        (0x1004, 0.5),
        (0x1005, 0.0),
        (0x2001, 1.0 / 9.0),
        (0x2002, 0.0),
        (0x2003, 0.25),
        (0x2004, 0.5),
        (0x3001, 1.0 / 9.0),
        (0x3002, 0.0),
        (0x3003, 0.25),
        (0x3004, 0.5),
        (0x3005, 0.0),
        (0x4001, 1.0 / 9.0),
        (0x4002, 0.0),
        (0x4003, 0.25),
        (0x4004, 0.5),
        (0x5001, 1.0 / 9.0),
        (0x5002, 0.0),
        (0x5003, 0.25),
        (0x5004, 0.5),
        (0x5005, 0.0),
    ];
    let changes = changes(
        declared
            .into_iter()
            .map(|(id, value)| {
                queue(
                    id,
                    if id == 0x1003 {
                        vec![(7, 0.75), (8, 0.125), (12, 0.625)]
                    } else {
                        vec![(0, value)]
                    },
                )
            })
            .collect(),
    );
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputParameterChanges = changes.as_ptr();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    let pcm = CURVE_PCM;
    for (i, sample) in pcm.iter().enumerate() {
        let normalized = match i {
            0..=7 => 0.25 + 0.5 * (i + 1) as f64 / 8.0,
            8..=12 => 0.125 + 0.5 * (i - 8) as f64 / 4.0,
            _ => 0.625,
        };
        let expected = f32::from(*sample) / 32768.0 * (4.0 * normalized) as f32;
        assert_eq!(
            (block.left[i], block.right[i]),
            (expected, expected),
            "signed ABI curve sample{i}"
        );
    }
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn malformed_standard_queues_preserve_accepted_sound_and_cursor_atomically() {
    use vst3::{ComPtr, ComWrapper};
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    start(&p, &a, 48000.0, 16);
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    let mut malformed: Vec<ComPtr<IParameterChanges>> = Vec::new();
    for points in [
        vec![(-1, 0.5)],
        vec![(16, 0.5)],
        vec![(0, -0.1)],
        vec![(0, 1.1)],
        vec![(0, f64::NAN)],
        vec![(0, f64::INFINITY)],
        vec![(0, 0.5), (0, 0.75)],
        vec![(2, 0.5), (1, 0.75)],
    ] {
        malformed.push(changes(vec![
            queue(0x1003, vec![(0, 0.0)]),
            queue(0x1004, points),
        ]));
    }
    malformed.push(changes(vec![queue(999, vec![(0, 0.5)])]));
    malformed.push(changes(vec![
        queue(0x1003, vec![(0, 0.0)]),
        queue(0x1003, vec![(1, 0.5)]),
    ]));
    malformed.push(changes(vec![queue(0x1003, vec![(0, 0.5)]); 24]));
    for count in [-1, 24] {
        malformed.push(
            ComWrapper::new(Changes {
                queues: vec![],
                count: Some(count),
                null: None,
            })
            .to_com_ptr::<IParameterChanges>()
            .unwrap(),
        );
    }
    malformed.push(
        ComWrapper::new(Changes {
            queues: vec![queue(0x1003, vec![(0, 0.0)])],
            count: None,
            null: Some(0),
        })
        .to_com_ptr::<IParameterChanges>()
        .unwrap(),
    );
    for count in [-1, 1025] {
        let q = ComWrapper::new(Queue {
            id: 0x1003,
            points: vec![],
            fail: None,
            count: Some(count),
        })
        .to_com_ptr::<IParamValueQueue>()
        .unwrap();
        malformed.push(changes(vec![q]));
    }
    let q = ComWrapper::new(Queue {
        id: 0x1004,
        points: vec![(0, 0.5), (1, 0.75)],
        fail: Some(1),
        count: None,
    })
    .to_com_ptr::<IParamValueQueue>()
    .unwrap();
    malformed.push(changes(vec![queue(0x1003, vec![(0, 0.0)]), q]));
    for (case, changes) in malformed.iter().enumerate() {
        let mut block = Block::new(16);
        let mut data = block.data();
        data.inputParameterChanges = changes.as_ptr();
        assert_eq!(
            unsafe { a.process(&mut data) },
            kInvalidArgument,
            "malformed queue {case}"
        );
        assert_eq!(block.left, [0.0; 16]);
        assert_eq!(block.right, [0.0; 16]);
    }
    let valid_but_unusable = changes(vec![queue(0x1003, vec![(0, 0.0)])]);
    let mut bad_output = Block::new(16);
    bad_output.bus.numChannels = 1;
    bad_output.right.fill(7.0);
    let mut data = bad_output.data();
    data.inputParameterChanges = valid_but_unusable.as_ptr();
    assert_eq!(
        unsafe { a.process(&mut data) },
        kInvalidArgument,
        "valid curve cannot commit before output validation"
    );
    assert_eq!(bad_output.left, [0.0; 16]);
    assert_eq!(bad_output.right, [7.0; 16]);
    let zero_bad = changes(vec![queue(0x1003, vec![(1, 0.0)])]);
    let mut zero = Block::new(0);
    let mut data = zero.data();
    data.inputParameterChanges = zero_bad.as_ptr();
    assert_eq!(
        unsafe { a.process(&mut data) },
        kInvalidArgument,
        "zero-sample point offset must be zero"
    );
    let no_queues = changes(vec![]);
    let mut data = zero.data();
    data.inputParameterChanges = no_queues.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    let empty = changes(vec![queue(0x1003, vec![])]);
    let mut continuation = Block::new(16);
    let mut data = continuation.data();
    data.inputParameterChanges = empty.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(
        (continuation.left[0], continuation.right[0]),
        (11869.0 / 32768.0, 11869.0 / 32768.0)
    );
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn combined_point_and_note_budget_accepts_1024_and_rejects_1025_without_advancing() {
    let bundle = Bundle::new();
    linear_sample(&bundle);
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    start(&p, &a, 48000.0, 1024);
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let accepted = changes(vec![queue(0x1003, (0..1023).map(|i| (i, 0.25)).collect())]);
    let mut block = Block::new(1024);
    let mut data = block.data();
    data.inputParameterChanges = accepted.as_ptr();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    for i in 0..1024 {
        let expected = (i as f32 - 24000.0) / 32768.0;
        assert_eq!((block.left[i], block.right[i]), (expected, expected));
    }
    let excess = changes(vec![queue(0x1003, (0..1024).map(|i| (i, 0.0)).collect())]);
    let mut data = block.data();
    data.inputParameterChanges = excess.as_ptr();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kInvalidArgument);
    assert!(block.left.iter().chain(&block.right).all(|v| *v == 0.0));
    let mut continuation = Block::new(16);
    assert_eq!(unsafe { a.process(&mut continuation.data()) }, kResultOk);
    assert_eq!(
        (continuation.left[0], continuation.right[0]),
        (-22976.0 / 32768.0, -22976.0 / 32768.0)
    );
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

#[test]
fn equal_frame_notes_keep_host_order_and_sample_the_new_onset_control() {
    let bundle = Bundle::new();
    let document = bundle.resources.join("instrument/advanced-drum-kit.yaml");
    let yaml = std::fs::read_to_string(&document).unwrap();
    assert!(yaml.contains("max_voices: 8"));
    std::fs::write(document, yaml.replace("max_voices: 8", "max_voices: 1")).unwrap();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    start(&p, &a, 48000.0, 16);
    let changes = changes(vec![queue(0x1002, vec![(4, 0.0), (5, 1.0)])]);
    // Snare soft ends at PCM -149; the final kick ends at PCM +105.
    // Reversing host order with one voice therefore changes signed output.
    let notes = events(vec![note_on(5, 38, 0.25), note_on(5, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputParameterChanges = changes.as_ptr();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { a.process(&mut data) }, kResultOk);
    assert_eq!(&block.left[..5], &[0.0; 5]);
    assert_eq!(
        (block.left[5], block.right[5]),
        (105.0 / 32768.0, 105.0 / 32768.0)
    );
    assert_eq!(&block.left[6..], &[0.0; 10]);
    assert_eq!(block.left, block.right);
    assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
    assert_eq!(unsafe { p.terminate() }, kResultOk);
}

const CURVE_PCM: [i16; 16] = [
    -16384, -358, 1554, 4137, -2136, 1664, 6324, 4571, -441, 6389, 2934, 5159, 7331, 10268, 4721,
    12612,
];

#[test]
fn standard_host_curves_are_invariant_under_equivalent_block_partitioning() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let whole = module.class::<IComponent>(&PROCESSOR);
    let wa = whole.cast::<IAudioProcessor>().unwrap();
    let split = module.class::<IComponent>(&PROCESSOR);
    let sa = split.cast::<IAudioProcessor>().unwrap();
    for p in [&whole, &split] {
        assert_eq!(unsafe { p.initialize(std::ptr::null_mut()) }, kResultOk);
    }
    start(&whole, &wa, 48000.0, 16);
    start(&split, &sa, 48000.0, 16);
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let one = changes(vec![queue(0x1003, vec![(15, 0.75)])]);
    let mut all = Block::new(16);
    let mut data = all.data();
    data.inputParameterChanges = one.as_ptr();
    data.inputEvents = notes.as_ptr();
    assert_eq!(unsafe { wa.process(&mut data) }, kResultOk);
    for (part, value) in [(0, 0.5), (1, 0.75)] {
        let points = changes(vec![queue(0x1003, vec![(7, value)])]);
        let mut block = Block::new(8);
        let mut data = block.data();
        data.inputParameterChanges = points.as_ptr();
        if part == 0 {
            data.inputEvents = notes.as_ptr();
        }
        assert_eq!(unsafe { sa.process(&mut data) }, kResultOk);
        for i in 0..8 {
            let frame = part * 8 + i;
            let expected =
                f32::from(CURVE_PCM[frame]) / 32768.0 * (1.0 + 2.0 * (frame + 1) as f32 / 16.0);
            assert_eq!(
                (
                    all.left[frame],
                    all.right[frame],
                    block.left[i],
                    block.right[i]
                ),
                (expected, expected, expected, expected),
                "ABI partition frame{frame}"
            );
        }
    }
    for (p, a) in [(&whole, &wa), (&split, &sa)] {
        assert_eq!(unsafe { a.setProcessing(0) }, kResultOk);
        assert_eq!(unsafe { p.terminate() }, kResultOk);
    }
}
