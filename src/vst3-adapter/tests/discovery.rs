#![cfg(all(target_os = "linux", target_arch = "x86_64"))]
mod common;
use common::*;
use vst3::{ComRef, Steinberg::*};

#[test]
fn dynamic_factory_exposes_distinct_stable_non_distributable_classes() {
    let bundle = Bundle::new();
    let module = bundle.load();
    let factory = module.factory();
    unsafe {
        assert_eq!(factory.countClasses(), 2);
        let mut vendor: PFactoryInfo = std::mem::zeroed();
        assert_eq!(factory.getFactoryInfo(&mut vendor), kResultOk);
        assert_eq!(ascii(&vendor.vendor), "Dandrum");
        assert_eq!(
            factory.getFactoryInfo(std::ptr::null_mut()),
            kInvalidArgument
        );
        let factory3 = factory
            .cast::<IPluginFactory3>()
            .expect("standard extended factory");
        assert_eq!(factory3.setHostContext(std::ptr::null_mut()), kResultOk);
        for (index, cid, category, name) in [
            (0, PROCESSOR, "Audio Module Class", "Dandrum Trigger"),
            (
                1,
                CONTROLLER,
                "Component Controller Class",
                "Dandrum Trigger Controller",
            ),
        ] {
            let mut info: PClassInfo = std::mem::zeroed();
            assert_eq!(factory.getClassInfo(index, &mut info), kResultOk);
            assert_eq!(info.cid, cid);
            assert_eq!(ascii(&info.category), category);
            assert_eq!(ascii(&info.name), name);
            let mut extended: PClassInfo2 = std::mem::zeroed();
            assert_eq!(factory3.getClassInfo2(index, &mut extended), kResultOk);
            assert_eq!(extended.cid, cid);
            assert_eq!(extended.classFlags, 0, "no distributable class contract");
            assert_eq!(ascii(&extended.name), name);
            assert_eq!(
                ascii(&extended.subCategories),
                if index == 0 { "Instrument|Sampler" } else { "" }
            );
            let mut unicode: PClassInfoW = std::mem::zeroed();
            assert_eq!(factory3.getClassInfoUnicode(index, &mut unicode), kResultOk);
            assert_eq!(unicode.cid, cid);
            assert_eq!(utf16(&unicode.name), name);
            assert_eq!(utf16(&unicode.vendor), "Dandrum");
        }
        for index in [-1, 2, i32::MAX] {
            let mut info: PClassInfo = std::mem::zeroed();
            info.cid = [7; 16];
            assert_eq!(factory.getClassInfo(index, &mut info), kInvalidArgument);
            assert_eq!(info.cid, [7; 16]);
            let mut extended: PClassInfo2 = std::mem::zeroed();
            assert_eq!(
                factory3.getClassInfo2(index, &mut extended),
                kInvalidArgument
            );
            let mut unicode: PClassInfoW = std::mem::zeroed();
            assert_eq!(
                factory3.getClassInfoUnicode(index, &mut unicode),
                kInvalidArgument
            );
        }
        assert_eq!(
            factory.getClassInfo(0, std::ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            factory3.getClassInfo2(0, std::ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            factory3.getClassInfoUnicode(0, std::ptr::null_mut()),
            kInvalidArgument
        );
        let unknown = [42; 16];
        let mut output = std::ptr::dangling_mut();
        assert_eq!(
            factory.createInstance(
                unknown.as_ptr(),
                <FUnknown as vst3::Interface>::IID.as_ptr().cast(),
                &mut output
            ),
            kNoInterface
        );
        assert!(output.is_null());
        for (cid, iid) in [
            (std::ptr::null(), unknown.as_ptr()),
            (unknown.as_ptr(), std::ptr::null()),
        ] {
            output = std::ptr::dangling_mut();
            assert_eq!(
                factory.createInstance(cid, iid, &mut output),
                kInvalidArgument
            );
            assert!(
                output.is_null(),
                "failed creation clears the owned-reference output"
            );
        }

        assert_eq!(
            factory.createInstance(std::ptr::null(), std::ptr::null(), std::ptr::null_mut()),
            kInvalidArgument
        );
        let unknown = ComRef::from_raw(factory.as_ptr())
            .unwrap()
            .cast::<FUnknown>()
            .unwrap();
        assert_eq!(unknown.cast::<IPluginFactory>().unwrap().countClasses(), 2);
    }
}

const PUBLIC: [(u32, &str, f64, f64, f64); 23] = [
    (0x1001, "drums.pitch_ratio", 0.125, 8.0, 1.0),
    (0x1002, "drums.start_offset", 0.0, 1.0, 0.0),
    (0x1003, "drums.level", 0.0, 4.0, 1.0),
    (0x1004, "drums.pan", -1.0, 1.0, 0.0),
    (0x1005, "drums.variation", 0.0, 1.0, 0.0),
    (0x2001, "drums.kick.pitch_ratio", 0.125, 8.0, 1.0),
    (0x2002, "drums.kick.start_offset", 0.0, 1.0, 0.0),
    (0x2003, "drums.kick.level", 0.0, 4.0, 1.0),
    (0x2004, "drums.kick.pan", -1.0, 1.0, 0.0),
    (0x3001, "drums.snare.pitch_ratio", 0.125, 8.0, 1.0),
    (0x3002, "drums.snare.start_offset", 0.0, 1.0, 0.0),
    (0x3003, "drums.snare.level", 0.0, 4.0, 1.0),
    (0x3004, "drums.snare.pan", -1.0, 1.0, 0.0),
    (0x3005, "drums.snare.variation", 0.0, 1.0, 0.0),
    (0x4001, "drums.closed_hat.pitch_ratio", 0.125, 8.0, 1.0),
    (0x4002, "drums.closed_hat.start_offset", 0.0, 1.0, 0.0),
    (0x4003, "drums.closed_hat.level", 0.0, 4.0, 1.0),
    (0x4004, "drums.closed_hat.pan", -1.0, 1.0, 0.0),
    (0x5001, "drums.open_hat.pitch_ratio", 0.125, 8.0, 1.0),
    (0x5002, "drums.open_hat.start_offset", 0.0, 1.0, 0.0),
    (0x5003, "drums.open_hat.level", 0.0, 4.0, 1.0),
    (0x5004, "drums.open_hat.pan", -1.0, 1.0, 0.0),
    (0x5005, "drums.open_hat.variation", 0.0, 1.0, 0.0),
];

#[test]
fn controller_projects_all_named_values_without_an_instrument_session() {
    use vst3::Steinberg::Vst::*;
    let bundle = Bundle::new();
    // Descriptors need no DSP/sample decoding. A controller implemented by
    // constructing another ParameterController would fail on this fixture.
    std::fs::write(
        bundle
            .resources
            .join("instrument/assets/advanced-drums.wav"),
        b"not a playable waveform",
    )
    .unwrap();
    let module = bundle.load();
    let controller = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert!(
            controller.cast::<IComponent>().is_none(),
            "separate controller class"
        );
        assert_eq!(controller.getParameterCount(), 0);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(
            controller.initialize(std::ptr::null_mut()),
            kResultFalse,
            "duplicate init preserves projection"
        );
        assert_eq!(controller.getParameterCount(), 23);
        for (index, (id, name, min, max, default)) in PUBLIC.iter().enumerate() {
            let mut info: ParameterInfo = std::mem::zeroed();
            assert_eq!(
                controller.getParameterInfo(index as i32, &mut info),
                kResultOk
            );
            assert_eq!(info.id, *id);
            assert_eq!(utf16(&info.title), *name);
            assert_eq!(info.stepCount, 0);
            assert_eq!(info.flags, ParameterInfo_::ParameterFlags_::kCanAutomate);
            let normalized_default = (default - min) / (max - min);
            assert!((info.defaultNormalizedValue - normalized_default).abs() < 1e-14);
            assert_eq!(controller.getParamNormalized(*id), normalized_default);
            for normalized in [0.0, normalized_default, (index + 1) as f64 / 32.0, 1.0] {
                let physical = min + (max - min) * normalized;
                assert!(
                    (controller.normalizedParamToPlain(*id, normalized) - physical).abs() < 1e-12
                );
                assert!(
                    (controller.plainParamToNormalized(*id, physical) - normalized).abs() < 1e-14
                );
                assert_eq!(controller.setParamNormalized(*id, normalized), kResultOk);
                assert_eq!(controller.getParamNormalized(*id), normalized);
                let mut text = [0; 128];
                assert_eq!(
                    controller.getParamStringByValue(*id, normalized, &mut text),
                    kResultOk
                );
                let displayed: f64 = utf16(&text).parse().unwrap();
                assert!((displayed - physical).abs() < 1e-12);
                let mut restored = -1.0;
                assert_eq!(
                    controller.getParamValueByString(*id, text.as_mut_ptr(), &mut restored),
                    kResultOk
                );
                assert!((restored - normalized).abs() < 1e-14);
            }
            for invalid in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.1, 1.1] {
                assert_eq!(
                    controller.setParamNormalized(*id, invalid),
                    kInvalidArgument
                );
                assert_eq!(controller.getParamNormalized(*id), 1.0);
                assert!(controller.normalizedParamToPlain(*id, invalid).is_nan());
            }
            for invalid in [f64::NAN, f64::INFINITY, *min - 1.0, *max + 1.0] {
                assert!(controller.plainParamToNormalized(*id, invalid).is_nan());
            }
        }
        let mut info: ParameterInfo = std::mem::zeroed();
        assert_eq!(controller.getParameterInfo(-1, &mut info), kInvalidArgument);
        assert_eq!(controller.getParameterInfo(23, &mut info), kInvalidArgument);
        assert_eq!(
            controller.getParameterInfo(0, std::ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            controller.setParamNormalized(0xffff_ffff, 0.5),
            kInvalidArgument
        );
        assert!(controller.normalizedParamToPlain(0xffff_ffff, 0.5).is_nan());
        assert!(controller.plainParamToNormalized(0xffff_ffff, 0.5).is_nan());
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(controller.getParameterCount(), 0);
    }
}

#[test]
fn missing_bundle_resources_fail_without_a_checkout_fallback_or_partial_owner() {
    use vst3::Steinberg::Vst::*;
    let bundle = Bundle::new();
    let document = bundle.resources.join("instrument/advanced-drum-kit.yaml");
    let yaml = std::fs::read(&document).unwrap();
    std::fs::remove_file(&document).unwrap();
    let module = bundle.load();
    let processor = module.class::<IComponent>(&PROCESSOR);
    let controller = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert!(processor.cast::<IEditController>().is_none());
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultFalse);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultFalse);
        assert_eq!(controller.getParameterCount(), 0);
        std::fs::write(&document, &yaml).unwrap();
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        let mut cid = [0; 16];
        assert_eq!(processor.getControllerClassId(&mut cid), kResultOk);
        assert_eq!(cid, CONTROLLER);
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultFalse);
        assert_eq!(processor.terminate(), kResultOk);
        assert_eq!(controller.terminate(), kResultOk);
    }
}

#[test]
fn named_ids_survive_document_reordering_and_reject_incompatible_surfaces() {
    use vst3::Steinberg::Vst::*;
    for change in ["reorder", "rename", "remove", "extra", "malformed", "wave"] {
        let bundle = Bundle::new();
        let document = bundle.resources.join("instrument/advanced-drum-kit.yaml");
        let mut yaml = std::fs::read_to_string(&document).unwrap();
        match change {
            "reorder" => {
                let a = "    - { name: drums.pitch_ratio, maps_to: pitch_ratio }";
                let b = "    - { name: drums.start_offset, maps_to: start_offset }";
                yaml = yaml.replace(&format!("{a}\n{b}"), &format!("{b}\n{a}"));
            }
            "rename" => {
                yaml = yaml.replace("name: drums.pitch_ratio,", "name: renamed.pitch_ratio,")
            }
            "remove" => {
                yaml = yaml.replace(
                    "    - { name: drums.pitch_ratio, maps_to: pitch_ratio }\n",
                    "",
                )
            }
            "extra" => {
                yaml = yaml.replace("ports:\n", "ports:\n  - { name: extra, direction: input, signal: control, channels: 1, default: 1, min: 0, max: 4, maps_to: player.group_1_level }\n");
                yaml = yaml.replace(
                    "  parameters:\n",
                    "  parameters:\n    - { name: extra, maps_to: extra }\n",
                );
            }
            "malformed" => yaml = "not: [valid".into(),
            "wave" => std::fs::remove_file(
                bundle
                    .resources
                    .join("instrument/assets/advanced-drums.wav"),
            )
            .unwrap(),
            _ => unreachable!(),
        }
        std::fs::write(document, yaml).unwrap();
        let module = bundle.load();
        let controller = module.class::<IEditController>(&CONTROLLER);
        let processor = module.class::<IComponent>(&PROCESSOR);
        unsafe {
            let expected = if change == "reorder" {
                kResultOk
            } else {
                kResultFalse
            };
            assert_eq!(
                controller.initialize(std::ptr::null_mut()),
                expected,
                "{change}"
            );
            assert_eq!(
                processor.initialize(std::ptr::null_mut()),
                expected,
                "{change}"
            );
            if change == "reorder" {
                for (index, (id, name, ..)) in PUBLIC.iter().enumerate() {
                    let mut info = std::mem::zeroed();
                    assert_eq!(
                        controller.getParameterInfo(index as i32, &mut info),
                        kResultOk
                    );
                    assert_eq!(info.id, *id);
                    assert_eq!(utf16(&info.title), *name);
                }
            } else {
                assert_eq!(controller.getParameterCount(), 0);
                assert_eq!(
                    processor
                        .getBusCount(MediaTypes_::kAudio as i32, BusDirections_::kOutput as i32),
                    0
                );
            }
            assert_eq!(processor.terminate(), kResultOk);
            assert_eq!(controller.terminate(), kResultOk);
        }
    }
}

#[test]
fn projection_lifecycle_and_text_errors_preserve_last_valid_value() {
    use vst3::Steinberg::Vst::*;
    let bundle = Bundle::new();
    let module = bundle.load();
    let controller = module.class::<IEditController>(&CONTROLLER);
    let processor = module.class::<IComponent>(&PROCESSOR);
    unsafe {
        let mut info = std::mem::zeroed();
        assert_eq!(controller.getParameterInfo(0, &mut info), kNotInitialized);
        assert_eq!(controller.setParamNormalized(0x1003, 0.25), kNotInitialized);
        assert!(controller.normalizedParamToPlain(0x1003, 0.25).is_nan());
        assert!(controller.plainParamToNormalized(0x1003, 1.0).is_nan());
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.setParamNormalized(0x1003, 0.375), kResultOk);
        assert_eq!(controller.getParamNormalized(u32::MAX), 0.0);
        let mut output = 123.0;
        for text in [
            vec![b'x' as u16, 0],
            vec![0xd800, 0],
            vec![b'1' as u16; 128],
            vec![b'9' as u16, 0],
        ] {
            let mut text = text;
            assert_eq!(
                controller.getParamValueByString(0x1003, text.as_mut_ptr(), &mut output),
                kInvalidArgument
            );
            assert_eq!(output, 123.0);
            assert_eq!(controller.getParamNormalized(0x1003), 0.375);
        }
        let mut text = [0; 128];
        assert_eq!(
            controller.getParamValueByString(0x1003, std::ptr::null_mut(), &mut output),
            kInvalidArgument
        );
        assert_eq!(
            controller.getParamValueByString(0x1003, text.as_mut_ptr(), std::ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            controller.getParamStringByValue(0x1003, 0.5, std::ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(
            controller.getParamStringByValue(u32::MAX, 0.5, &mut text),
            kInvalidArgument
        );
        assert_eq!(
            processor.getControllerClassId(std::ptr::null_mut()),
            kInvalidArgument
        );
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(processor.setIoMode(0), kResultOk);
        assert_eq!(processor.setIoMode(1), kResultOk);
        assert_eq!(processor.setIoMode(2), kResultOk);
        assert_eq!(processor.setIoMode(3), kInvalidArgument);
        for (media, direction, channels, name) in [
            (
                MediaTypes_::kAudio as i32,
                BusDirections_::kOutput as i32,
                2,
                "Stereo Output",
            ),
            (
                MediaTypes_::kEvent as i32,
                BusDirections_::kInput as i32,
                16,
                "MIDI Input",
            ),
        ] {
            assert_eq!(processor.getBusCount(media, direction), 1);
            let mut bus: BusInfo = std::mem::zeroed();
            assert_eq!(
                processor.getBusInfo(media, direction, 0, &mut bus),
                kResultOk
            );
            assert_eq!(bus.mediaType, media);
            assert_eq!(bus.direction, direction);
            assert_eq!(bus.channelCount, channels);
            assert_eq!(utf16(&bus.name), name);
            assert_eq!(
                processor.getBusInfo(media, direction, 1, &mut bus),
                kInvalidArgument
            );
            assert_eq!(
                processor.getBusInfo(media, direction, 0, std::ptr::null_mut()),
                kInvalidArgument
            );
        }
        let mut bus = std::mem::zeroed();
        assert_eq!(
            processor.getBusCount(MediaTypes_::kAudio as i32, BusDirections_::kInput as i32),
            0
        );
        assert_eq!(
            processor.getBusInfo(
                MediaTypes_::kAudio as i32,
                BusDirections_::kInput as i32,
                0,
                &mut bus
            ),
            kInvalidArgument
        );
        assert_eq!(processor.terminate(), kResultOk);
        assert_eq!(processor.getBusInfo(0, 1, 0, &mut bus), kNotInitialized);
        // VST3 permits restarting the same terminated processor. Its fresh
        // service/setup/audio contract is asserted by the service owner test.
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(processor.terminate(), kResultOk);
        assert_eq!(controller.terminate(), kResultOk);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultFalse);
        assert_eq!(controller.getParameterCount(), 0);
    }
}

#[test]
fn wrong_bundle_layout_and_unsupported_interfaces_clear_owned_outputs() {
    use vst3::Steinberg::Vst::*;
    let mut bundle = Bundle::new();
    {
        let module = bundle.load();
        let factory = module.factory();
        let mut output = std::ptr::dangling_mut();
        unsafe {
            assert_eq!(
                factory.createInstance(
                    PROCESSOR.as_ptr(),
                    <IEditController as vst3::Interface>::IID.as_ptr().cast(),
                    &mut output
                ),
                kNoInterface
            );
            assert!(output.is_null());
            assert_eq!(
                factory.createInstance(
                    CONTROLLER.as_ptr(),
                    <IComponent as vst3::Interface>::IID.as_ptr().cast(),
                    &mut output
                ),
                kNoInterface
            );
            assert!(output.is_null());
        }
    }
    let new_binary = bundle
        .binary
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("wrong-architecture/DandrumTrigger.so");
    std::fs::create_dir_all(new_binary.parent().unwrap()).unwrap();
    std::fs::rename(&bundle.binary, &new_binary).unwrap();
    bundle.binary = new_binary;
    let module = bundle.load();
    let factory = module.factory();
    let mut output = std::ptr::dangling_mut();
    unsafe {
        assert_eq!(
            factory.createInstance(
                PROCESSOR.as_ptr(),
                <IComponent as vst3::Interface>::IID.as_ptr().cast(),
                &mut output
            ),
            kResultFalse
        );
        assert!(output.is_null());
    }
}

#[test]
fn foreign_thread_control_calls_reject_without_mutating_live_components() {
    use vst3::Steinberg::Vst::*;
    let bundle = Bundle::new();
    let module = bundle.load();
    let processor = module.class::<IComponent>(&PROCESSOR);
    let controller = module.class::<IEditController>(&CONTROLLER);
    unsafe {
        assert_eq!(processor.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.initialize(std::ptr::null_mut()), kResultOk);
        assert_eq!(controller.setParamNormalized(0x1003, 0.375), kResultOk);
    }
    let processor_address = processor.as_ptr() as usize;
    let controller_address = controller.as_ptr() as usize;
    std::thread::spawn(move || {
        // SAFETY: the owning thread retains both references and waits for this
        // thread. These intentionally invalid-affinity calls must reject before
        // accessing affine fields; references are not released on this thread.
        unsafe {
            let p = ComRef::from_raw(processor_address as *mut IComponent).unwrap();
            let c = ComRef::from_raw(controller_address as *mut IEditController).unwrap();
            assert_eq!(p.initialize(std::ptr::null_mut()), kResultFalse);
            assert_eq!(p.terminate(), kResultFalse);
            assert_eq!(p.setIoMode(0), kInvalidArgument);
            assert_eq!(p.getBusCount(0, 1), 0);
            let mut bus = std::mem::zeroed();
            assert_eq!(p.getBusInfo(0, 1, 0, &mut bus), kNotInitialized);
            assert_eq!(c.initialize(std::ptr::null_mut()), kResultFalse);
            assert_eq!(c.terminate(), kResultFalse);
            assert_eq!(c.getParameterCount(), 0);
            let mut info = std::mem::zeroed();
            assert_eq!(c.getParameterInfo(0, &mut info), kResultFalse);
            assert_eq!(c.setParamNormalized(0x1003, 0.9), kResultFalse);
            assert_eq!(c.getParamNormalized(0x1003), 0.0);
            assert!(c.normalizedParamToPlain(0x1003, 0.25).is_nan());
            assert!(c.plainParamToNormalized(0x1003, 1.0).is_nan());
            assert_eq!(c.setComponentHandler(std::ptr::null_mut()), kResultFalse);
        }
    })
    .join()
    .unwrap();
    unsafe {
        assert_eq!(controller.getParamNormalized(0x1003), 0.375);
        assert_eq!(controller.getParameterCount(), 23);
        assert_eq!(processor.getBusCount(0, 1), 1);
        assert_eq!(
            controller.setComponentHandler(std::ptr::null_mut()),
            kResultOk
        );
        assert_eq!(processor.terminate(), kResultOk);
        assert_eq!(controller.terminate(), kResultOk);
    }
}

#[test]
fn processor_declares_no_unused_optional_process_context_fields() {
    use vst3::Steinberg::Vst::{
        IComponent, IProcessContextRequirements, IProcessContextRequirementsTrait,
    };
    let bundle = Bundle::new();
    let module = bundle.load();
    let p = module.class::<IComponent>(&PROCESSOR);
    let requirements = p
        .cast::<IProcessContextRequirements>()
        .expect("SDK3.7+ mandatory process context interface");
    assert_eq!(
        unsafe { requirements.getProcessContextRequirements() },
        0,
        "Trigger uses setup clock and sample-offset input, not host context fields"
    );
}
