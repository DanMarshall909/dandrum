use vst3::{
    Class, ComPtr, ComWrapper,
    Steinberg::{Vst::*, *},
};
pub struct Queue {
    pub id: u32,
    pub points: Vec<(i32, f64)>,
    pub fail: Option<i32>,
    pub count: Option<i32>,
}
impl Class for Queue {
    type Interfaces = (IParamValueQueue,);
}
impl IParamValueQueueTrait for Queue {
    unsafe fn getParameterId(&self) -> u32 {
        self.id
    }
    unsafe fn getPointCount(&self) -> i32 {
        self.count.unwrap_or(self.points.len() as i32)
    }
    unsafe fn getPoint(&self, index: i32, offset: *mut i32, value: *mut f64) -> tresult {
        if self.fail == Some(index) {
            return kResultFalse;
        }
        let Some(&(frame, point)) = usize::try_from(index).ok().and_then(|i| self.points.get(i))
        else {
            return kInvalidArgument;
        };
        assert!(!offset.is_null() && !value.is_null());
        unsafe {
            *offset = frame;
            *value = point;
        }
        kResultOk
    }
    unsafe fn addPoint(&self, _: i32, _: f64, _: *mut i32) -> tresult {
        kNotImplemented
    }
}
pub struct Changes {
    pub queues: Vec<ComPtr<IParamValueQueue>>,
    pub count: Option<i32>,
    pub null: Option<i32>,
}
impl Class for Changes {
    type Interfaces = (IParameterChanges,);
}
impl IParameterChangesTrait for Changes {
    unsafe fn getParameterCount(&self) -> i32 {
        self.count.unwrap_or(self.queues.len() as i32)
    }
    unsafe fn getParameterData(&self, index: i32) -> *mut IParamValueQueue {
        if self.null == Some(index) {
            return std::ptr::null_mut();
        }
        self.queues
            .get(index as usize)
            .map_or(std::ptr::null_mut(), |q| q.as_ptr())
    }
    unsafe fn addParameterData(&self, _: *const u32, _: *mut i32) -> *mut IParamValueQueue {
        std::ptr::null_mut()
    }
}
pub fn queue(id: u32, points: Vec<(i32, f64)>) -> ComPtr<IParamValueQueue> {
    ComWrapper::new(Queue {
        id,
        points,
        fail: None,
        count: None,
    })
    .to_com_ptr::<IParamValueQueue>()
    .unwrap()
}
pub fn changes(queues: Vec<ComPtr<IParamValueQueue>>) -> ComPtr<IParameterChanges> {
    ComWrapper::new(Changes {
        queues,
        count: None,
        null: None,
    })
    .to_com_ptr::<IParameterChanges>()
    .unwrap()
}
