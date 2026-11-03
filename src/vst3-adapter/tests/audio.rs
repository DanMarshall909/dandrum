#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod common;
use common::{audio::*, *};
use vst3::Steinberg::{Vst::*, *};
#[test]
fn real_processor_renders_literal_signed_stereo_at_the_midi_offset_and_resets_voices() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let audio = p
        .cast::<IAudioProcessor>()
        .expect("processor exposes the standard audio interface");
    unsafe {
        let mut stereo = 3;
        assert_eq!(
            audio.setBusArrangements(std::ptr::null_mut(), 0, &mut stereo, 1),
            kResultOk
        );
        assert_eq!(audio.canProcessSampleSize(0), kResultOk);
        assert_eq!(audio.canProcessSampleSize(1), kResultFalse);
        assert_eq!(audio.getLatencySamples(), 0);
        assert_eq!(audio.setupProcessing(&mut setup(48_000.0, 16)), kResultOk);
        assert_eq!(p.activateBus(0, 1, 0, 1), kResultOk);
        assert_eq!(p.activateBus(1, 0, 0, 1), kResultOk);
        assert_eq!(p.setActive(1), kResultOk);
        assert_eq!(audio.setProcessing(1), kResultOk);
        let notes = events(vec![note_on(5, 36, 1.0)]);
        let mut block = Block::new(16);
        let mut data = block.data();
        data.inputEvents = notes.as_ptr();
        assert_eq!(audio.process(&mut data), kResultOk);
        let pcm: [i16; 11] = [
            -16384, -358, 1554, 4137, -2136, 1664, 6324, 4571, -441, 6389, 2934,
        ];
        for channel in [&block.left, &block.right] {
            assert_eq!(&channel[..5], &[0.0; 5]);
            for (i, sample) in pcm.iter().enumerate() {
                assert_eq!(
                    channel[i + 5],
                    f32::from(*sample) / 32768.0,
                    "signed source frame{i}"
                );
            }
        }
        assert_eq!(block.bus.silenceFlags, 0);
        assert_eq!(audio.setProcessing(0), kResultOk);
        assert_eq!(audio.setProcessing(1), kResultOk);
        let mut silent = Block::new(16);
        assert_eq!(audio.process(&mut silent.data()), kResultOk);
        assert_eq!(silent.left, [0.0; 16]);
        assert_eq!(silent.right, [0.0; 16]);
        assert_eq!(silent.bus.silenceFlags, 3);
        assert_eq!(audio.setProcessing(0), kResultOk);
        assert_eq!(p.setActive(0), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
}

#[test]
fn rejected_configuration_and_malformed_batches_preserve_the_sounding_cursor() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    unsafe {
        assert_eq!(a.setupProcessing(&mut setup(48_000.0, 16)), kNotInitialized);
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    start(&p, &a, 48_000.0, 16);
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut first = Block::new(16);
    let mut data = first.data();
    data.inputEvents = notes.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
    }
    for rate in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        unsafe {
            assert_eq!(a.setupProcessing(&mut setup(rate, 16)), kInvalidArgument);
        }
    }
    for frames in [-1, 0, 16385] {
        unsafe {
            assert_eq!(
                a.setupProcessing(&mut setup(48_000.0, frames)),
                kInvalidArgument
            );
        }
    }
    unsafe {
        assert_eq!(a.setupProcessing(&mut setup(44_100.0, 16)), kResultFalse);
        assert_eq!(p.setActive(0), kResultFalse);
        assert_eq!(a.setupProcessing(std::ptr::null_mut()), kInvalidArgument);
    }
    let mut batches = vec![
        vec![note_off(0, 36), note_on(16, 36, 1.0)],
        vec![note_on(3, 36, 1.0), note_off(2, 36)],
        vec![note_on(-1, 36, 1.0)],
        vec![note_on(0, 128, 1.0)],
        vec![note_on(0, 36, f32::NAN)],
        vec![note_on(0, 36, 1.1)],
    ];
    let mut bad = note_on(0, 36, 1.0);
    bad.busIndex = 1;
    batches.push(vec![bad]);
    let mut bad = note_on(0, 36, 1.0);
    bad.__field0.noteOn.channel = 16;
    batches.push(vec![bad]);
    for (pitch, channel, velocity, tuning) in [
        (128, 0, 0.0, 0.0),
        (36, 16, 0.0, 0.0),
        (36, 0, f32::NAN, 0.0),
        (36, 0, 0.0, 1.0),
    ] {
        let mut bad = note_off(0, pitch);
        bad.__field0.noteOff.channel = channel;
        bad.__field0.noteOff.velocity = velocity;
        bad.__field0.noteOff.tuning = tuning;
        batches.push(vec![bad]);
    }
    for batch in batches {
        let notes = events(batch);
        let mut block = Block::new(16);
        let mut data = block.data();
        data.inputEvents = notes.as_ptr();
        unsafe {
            assert_eq!(a.process(&mut data), kInvalidArgument);
        }
        assert_eq!(block.left, [0.0; 16]);
        assert_eq!(block.right, [0.0; 16]);
    }
    let failure = vst3::ComWrapper::new(Events {
        entries: vec![note_off(0, 36), note_on(1, 36, 1.0)],
        fail: Some(1),
    })
    .to_com_ptr::<IEventList>()
    .unwrap();
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = failure.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kInvalidArgument);
    }
    assert_eq!(block.left, [0.0; 16]);
    let mut block = Block::new(16);
    unsafe {
        assert_eq!(a.process(&mut block.data()), kResultOk);
    }
    let expected: [i16; 16] = [
        11869, 7042, 11000, 9960, 7319, 6682, 6798, 9639, 8245, 14213, 13425, 11883, 11177, 16190,
        18258, 11539,
    ];
    for channel in [&block.left, &block.right] {
        for (i, value) in expected.iter().enumerate() {
            assert_eq!(
                channel[i],
                f32::from(*value) / 32768.0,
                "preserved frame{}",
                16 + i
            );
        }
    }
    unsafe {
        assert_eq!(a.setProcessing(0), kResultOk);
        assert_eq!(p.setActive(0), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
}

#[test]
fn fractional_clocks_zero_sample_flush_and_two_audio_instances_keep_independent_time() {
    let bundle = Bundle::new();
    linear_sample(&bundle);
    let module = bundle.load();
    let first = module.class::<IComponent>(&PROCESSOR);
    let second = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(first.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(second.initialize(std::ptr::null_mut()), kResultOk);
    }
    let a = first.cast::<IAudioProcessor>().unwrap();
    let b = second.cast::<IAudioProcessor>().unwrap();
    start(&first, &a, 1234.5678, 16);
    start(&second, &b, 48_000.5, 16);
    let notes = events(vec![note_on(3, 36, 1.0)]);
    for (processor, rate) in [(&a, 1234.5678), (&b, 48_000.5)] {
        let mut block = Block::new(16);
        let mut data = block.data();
        data.inputEvents = notes.as_ptr();
        unsafe {
            assert_eq!(processor.process(&mut data), kResultOk);
        }
        for channel in [&block.left, &block.right] {
            assert_eq!(&channel[..3], &[0.0; 3]);
            assert_eq!(channel[3], -24000.0 / 32768.0);
            for (frame, value) in channel[3..].iter().enumerate() {
                let position = frame as f64 * 48_000.0 / rate;
                let expected = ((-24000.0 + position) / 32768.0) as f32;
                assert!(
                    (*value - expected).abs() < 2e-7,
                    "{rate} frame{frame}: {value} != {expected}"
                );
            }
        }
    }
    let flush_notes = events(vec![note_on(0, 36, 1.0)]);
    let mut flush = Block::new(0);
    let mut data = flush.data();
    data.inputEvents = flush_notes.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
    }
    for (processor, rate) in [(&a, 1234.5678), (&b, 48_000.5)] {
        let mut block = Block::new(16);
        unsafe {
            assert_eq!(processor.process(&mut block.data()), kResultOk);
        }
        for (frame, value) in block.left.iter().enumerate() {
            let expected = ((-24000.0 + (13 + frame) as f64 * 48_000.0 / rate) / 32768.0) as f32;
            assert!(
                (*value - expected).abs() < 2e-7,
                "independent continuation {rate} frame{frame}"
            );
        }
    }
    unsafe {
        assert_eq!(a.setProcessing(0), kResultOk);
        assert_eq!(b.setProcessing(0), kResultOk);
        assert_eq!(first.terminate(), kResultOk);
        assert_eq!(second.terminate(), kResultOk);
    }
}

#[test]
fn invalid_output_layouts_clear_only_accessible_planes_and_preserve_pending_notes() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let a = p.cast::<IAudioProcessor>().unwrap();
    start(&p, &a, 48_000.0, 16);
    for fault in [
        "channels",
        "null",
        "pointer_array",
        "misaligned",
        "outputs_null",
        "overlap",
        "size64",
        "negative",
        "inputs",
        "mode",
        "oversized",
    ] {
        let mut block = Block::new(16);
        let mut data = block.data();
        match fault {
            "channels" => block.bus.numChannels = 1,
            "null" => block.pointers[0] = std::ptr::null_mut(),
            "pointer_array" => block.bus.__field0.channelBuffers32 = std::ptr::null_mut(),
            "misaligned" => {
                block.pointers[0] = block.left.as_mut_ptr().cast::<u8>().wrapping_add(1).cast()
            }
            "outputs_null" => data.outputs = std::ptr::null_mut(),
            "overlap" => block.pointers[1] = block.pointers[0],
            "size64" => {
                data.symbolicSampleSize = 2;
            }
            "negative" => data.numSamples = -1,
            "inputs" => data.numInputs = 1,
            "mode" => data.processMode = 3,
            "oversized" => {
                data.numSamples = 17;
                block.left.push(123.0);
                block.right.push(123.0);
                block.pointers[0] = block.left.as_mut_ptr();
                block.pointers[1] = block.right.as_mut_ptr();
            }
            _ => unreachable!(),
        }
        unsafe {
            assert_eq!(a.process(&mut data), kInvalidArgument, "{fault}");
        }
        if matches!(fault, "channels" | "inputs" | "mode" | "oversized") {
            assert_eq!(block.left[0], 0.0);
        }
    }
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
    }
    assert_eq!((block.left[0], block.right[0]), (-0.5, -0.5));
    unsafe {
        assert_eq!(a.setProcessing(0), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
}

#[test]
fn declared_note_capacity_is_atomic_and_preserves_the_sounding_cursor() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let a = p.cast::<IAudioProcessor>().unwrap();
    start(&p, &a, 48_000.0, 16);
    let mut batch = vec![note_off(0, 36); 1024];
    batch[1023] = note_on(0, 36, 1.0);
    let notes = events(batch);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    unsafe {
        assert_eq!(
            a.process(&mut data),
            kResultOk,
            "all1024 admitted notes must fit a short callback"
        );
    }
    assert_eq!((block.left[0], block.right[0]), (-0.5, -0.5));
    let excess = events(vec![note_off(0, 36); 1025]);
    let mut bad = Block::new(16);
    let mut data = bad.data();
    data.inputEvents = excess.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kInvalidArgument);
    }
    assert_eq!(bad.left, [0.0; 16]);
    let mut continued = Block::new(16);
    unsafe {
        assert_eq!(a.process(&mut continued.data()), kResultOk);
    }
    assert_eq!(
        (continued.left[0], continued.right[0]),
        (11869.0 / 32768.0, 11869.0 / 32768.0)
    );
    unsafe {
        assert_eq!(a.setProcessing(0), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
}

#[test]
fn processing_instance_rejects_teardown_without_touching_the_sounding_voice() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
    }
    let a = p.cast::<IAudioProcessor>().unwrap();
    start(&p, &a, 48_000.0, 16);
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut block = Block::new(16);
    let mut data = block.data();
    data.inputEvents = notes.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
        assert_eq!(
            p.terminate(),
            kResultFalse,
            "processing must end before releasing its endpoint"
        );
    }
    let mut block = Block::new(16);
    unsafe {
        assert_eq!(a.process(&mut block.data()), kResultOk);
    }
    assert_eq!(block.left[0], 11869.0 / 32768.0);
    unsafe {
        assert_eq!(a.setProcessing(0), kResultOk);
        assert_eq!(p.terminate(), kResultOk);
    }
}

#[test]
fn bus_lifecycle_and_unsupported_event_policies_are_explicit() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let a = p.cast::<IAudioProcessor>().unwrap();
    let mut block = Block::new(16);
    unsafe {
        assert_eq!(a.process(&mut block.data()), kNotInitialized);
        assert_eq!(a.setProcessing(1), kNotInitialized);
        assert_eq!(p.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(a.setProcessing(1), kNotInitialized);
    }
    let mut arrangement = 99;
    unsafe {
        assert_eq!(a.getBusArrangement(1, 0, &mut arrangement), kResultOk);
        assert_eq!(arrangement, 3);
        assert_eq!(
            a.getBusArrangement(0, 0, &mut arrangement),
            kInvalidArgument
        );
        assert_eq!(
            a.getBusArrangement(1, 1, &mut arrangement),
            kInvalidArgument
        );
        assert_eq!(
            a.getBusArrangement(1, 0, std::ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            a.setBusArrangements(std::ptr::null_mut(), 1, &mut arrangement, 1),
            kInvalidArgument
        );
        arrangement = 99;
        assert_eq!(
            a.setBusArrangements(std::ptr::null_mut(), 0, &mut arrangement, 1),
            kResultFalse
        );
        assert_eq!(
            a.setBusArrangements(std::ptr::null_mut(), 0, std::ptr::null_mut(), 1),
            kInvalidArgument
        );
        assert_eq!(a.getTailSamples(), u32::MAX);
        assert_eq!(p.activateBus(0, 0, 0, 1), kInvalidArgument);
        assert_eq!(p.activateBus(1, 0, 1, 1), kInvalidArgument);
        assert_eq!(p.activateBus(0, 1, 0, 2), kInvalidArgument);
        assert_eq!(p.setActive(2), kInvalidArgument);
        assert_eq!(a.setProcessing(2), kInvalidArgument);
    }
    start(&p, &a, 48_000.0, 16);
    unsafe {
        assert_eq!(p.activateBus(1, 0, 0, 0), kResultOk);
    }
    let notes = events(vec![note_on(0, 36, 1.0)]);
    let mut silent = Block::new(16);
    let mut data = silent.data();
    data.inputEvents = notes.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
    }
    assert_eq!(silent.left, [0.0; 16]);
    unsafe {
        assert_eq!(p.activateBus(1, 0, 0, 1), kResultOk);
        assert_eq!(p.activateBus(0, 1, 0, 0), kResultOk);
    }
    let mut data = silent.data();
    data.inputEvents = notes.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
        assert_eq!(p.activateBus(0, 1, 0, 1), kResultOk);
    }
    let mut unsupported = note_on(0, 36, 1.0);
    unsupported.r#type = Event_::EventTypes_::kPolyPressureEvent as u16;
    let ignored = events(vec![unsupported]);
    let mut data = silent.data();
    data.inputEvents = ignored.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
    }
    assert_eq!(silent.left, [0.0; 16]);
    let mut left = [123.0f64; 16];
    let mut right = [123.0f64; 16];
    let mut pointers = [left.as_mut_ptr(), right.as_mut_ptr()];
    let mut bus = AudioBusBuffers {
        numChannels: 2,
        silenceFlags: 0,
        __field0: AudioBusBuffers__type0 {
            channelBuffers64: pointers.as_mut_ptr(),
        },
    };
    let mut data = silent.data();
    data.symbolicSampleSize = 1;
    data.outputs = &mut bus;
    unsafe {
        assert_eq!(a.process(&mut data), kInvalidArgument);
    }
    assert_eq!(left, [0.0; 16]);
    assert_eq!(right, [0.0; 16]);
    let mut data = silent.data();
    data.outputs = std::ptr::null_mut();
    unsafe {
        assert_eq!(a.process(&mut data), kInvalidArgument);
    }
    let zero_velocity = events(vec![note_on(0, 36, 0.0)]);
    let mut data = silent.data();
    data.inputEvents = zero_velocity.as_ptr();
    unsafe {
        assert_eq!(a.process(&mut data), kResultOk);
    }
    assert_eq!(silent.left, [0.0; 16]);
    unsafe {
        assert_eq!(a.setProcessing(0), kResultOk);
        assert_eq!(p.setActive(0), kResultOk);
        assert_eq!(a.process(&mut silent.data()), kNotInitialized);
        assert_eq!(p.terminate(), kResultOk);
    }
}
