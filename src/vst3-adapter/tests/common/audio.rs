use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};
pub struct Events {
    pub entries: Vec<Event>,
    pub fail: Option<i32>,
}
impl Class for Events {
    type Interfaces = (IEventList,);
}
impl IEventListTrait for Events {
    unsafe fn getEventCount(&self) -> i32 {
        self.entries.len() as i32
    }
    unsafe fn getEvent(&self, index: i32, event: *mut Event) -> tresult {
        if self.fail == Some(index) || event.is_null() {
            return kResultFalse;
        }
        let Some(value) = usize::try_from(index)
            .ok()
            .and_then(|i| self.entries.get(i))
        else {
            return kInvalidArgument;
        };
        unsafe {
            *event = *value;
        }
        kResultOk
    }
    unsafe fn addEvent(&self, _event: *mut Event) -> tresult {
        kNotImplemented
    }
}
pub fn events(entries: Vec<Event>) -> ComPtr<IEventList> {
    ComWrapper::new(Events {
        entries,
        fail: None,
    })
    .to_com_ptr::<IEventList>()
    .unwrap()
}
pub fn note_on(frame: i32, pitch: i16, velocity: f32) -> Event {
    Event {
        busIndex: 0,
        sampleOffset: frame,
        ppqPosition: 0.0,
        flags: 0,
        r#type: Event_::EventTypes_::kNoteOnEvent as u16,
        __field0: Event__type0 {
            noteOn: NoteOnEvent {
                channel: 0,
                pitch,
                tuning: 0.0,
                velocity,
                length: 0,
                noteId: -1,
            },
        },
    }
}
pub fn note_off(frame: i32, pitch: i16) -> Event {
    Event {
        busIndex: 0,
        sampleOffset: frame,
        ppqPosition: 0.0,
        flags: 0,
        r#type: Event_::EventTypes_::kNoteOffEvent as u16,
        __field0: Event__type0 {
            noteOff: NoteOffEvent {
                channel: 0,
                pitch,
                velocity: 0.0,
                noteId: -1,
                tuning: 0.0,
            },
        },
    }
}
pub struct Block {
    pub left: Vec<f32>,
    pub right: Vec<f32>,
    pub pointers: Box<[*mut f32; 2]>,
    pub bus: Box<AudioBusBuffers>,
}
impl Block {
    pub fn new(frames: usize) -> Self {
        let mut left = vec![123.0; frames];
        let mut right = vec![123.0; frames];
        let mut pointers = Box::new([left.as_mut_ptr(), right.as_mut_ptr()]);
        let bus = Box::new(AudioBusBuffers {
            numChannels: 2,
            silenceFlags: 0,
            __field0: AudioBusBuffers__type0 {
                channelBuffers32: pointers.as_mut_ptr(),
            },
        });
        Self {
            left,
            right,
            pointers,
            bus,
        }
    }
    pub fn data(&mut self) -> ProcessData {
        ProcessData {
            processMode: 0,
            symbolicSampleSize: 0,
            numSamples: self.left.len() as i32,
            numInputs: 0,
            numOutputs: 1,
            inputs: std::ptr::null_mut(),
            outputs: self.bus.as_mut(),
            inputParameterChanges: std::ptr::null_mut(),
            outputParameterChanges: std::ptr::null_mut(),
            inputEvents: std::ptr::null_mut(),
            outputEvents: std::ptr::null_mut(),
            processContext: std::ptr::null_mut(),
        }
    }
}
pub fn setup(rate: f64, frames: i32) -> ProcessSetup {
    ProcessSetup {
        processMode: 0,
        symbolicSampleSize: 0,
        maxSamplesPerBlock: frames,
        sampleRate: rate,
    }
}

pub fn start(
    component: &ComPtr<IComponent>,
    processor: &ComPtr<IAudioProcessor>,
    rate: f64,
    frames: i32,
) {
    unsafe {
        assert_eq!(
            processor.setupProcessing(&mut setup(rate, frames)),
            kResultOk
        );
        assert_eq!(component.setActive(1), kResultOk);
        assert_eq!(processor.setProcessing(1), kResultOk);
    }
}
pub fn linear_sample(bundle: &super::Bundle) {
    use std::io::Write;
    let mut file = std::fs::File::create(
        bundle
            .resources
            .join("instrument/assets/advanced-drums.wav"),
    )
    .unwrap();
    let bytes = 51000u32 * 2;
    for part in [
        b"RIFF".to_vec(),
        (36 + bytes).to_le_bytes().to_vec(),
        b"WAVEfmt ".to_vec(),
        16u32.to_le_bytes().to_vec(),
        1u16.to_le_bytes().to_vec(),
        1u16.to_le_bytes().to_vec(),
        48000u32.to_le_bytes().to_vec(),
        96000u32.to_le_bytes().to_vec(),
        2u16.to_le_bytes().to_vec(),
        16u16.to_le_bytes().to_vec(),
        b"data".to_vec(),
        bytes.to_le_bytes().to_vec(),
    ] {
        file.write_all(&part).unwrap();
    }
    for frame in 0..51000 {
        file.write_all(&((frame - 24000) as i16).to_le_bytes())
            .unwrap();
    }
}
