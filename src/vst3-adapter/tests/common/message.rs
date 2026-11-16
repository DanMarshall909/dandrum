//! Host-owned standard-message fixture; faults affect only foreign ABI outputs.
use std::ffi::{CString, c_void};
use vst3::{
    Class, ComPtr, ComRef, ComWrapper,
    Steinberg::{Vst::*, *},
};
pub struct Input {
    pub id: Option<CString>,
    pub attributes: Option<ComPtr<IAttributeList>>,
}
impl Class for Input {
    type Interfaces = (IMessage,);
}
impl IMessageTrait for Input {
    unsafe fn getMessageID(&self) -> FIDString {
        self.id.as_ref().map_or(std::ptr::null(), |id| id.as_ptr())
    }
    unsafe fn setMessageID(&self, _id: FIDString) {}
    unsafe fn getAttributes(&self) -> *mut IAttributeList {
        self.attributes
            .as_ref()
            .map_or(std::ptr::null_mut(), ComPtr::as_ptr)
    }
}
pub struct Blob {
    pub bytes: Vec<u8>,
    pub reported: Option<u32>,
    pub null_data: bool,
    pub fail: bool,
}
impl Class for Blob {
    type Interfaces = (IAttributeList,);
}
impl IAttributeListTrait for Blob {
    unsafe fn setInt(&self, _id: *const i8, _v: i64) -> tresult {
        kNotImplemented
    }
    unsafe fn getInt(&self, _id: *const i8, _v: *mut i64) -> tresult {
        kResultFalse
    }
    unsafe fn setFloat(&self, _id: *const i8, _v: f64) -> tresult {
        kNotImplemented
    }
    unsafe fn getFloat(&self, _id: *const i8, _v: *mut f64) -> tresult {
        kResultFalse
    }
    unsafe fn setString(&self, _id: *const i8, _v: *const u16) -> tresult {
        kNotImplemented
    }
    unsafe fn getString(&self, _id: *const i8, _v: *mut u16, _n: u32) -> tresult {
        kResultFalse
    }
    unsafe fn setBinary(&self, _id: *const i8, _v: *const c_void, _n: u32) -> tresult {
        kNotImplemented
    }
    unsafe fn getBinary(
        &self,
        _id: *const i8,
        data: *mut *const c_void,
        length: *mut u32,
    ) -> tresult {
        if self.fail {
            return kResultFalse;
        }
        unsafe {
            *data = if self.null_data {
                std::ptr::null()
            } else {
                self.bytes.as_ptr().cast()
            };
            *length = self.reported.unwrap_or(self.bytes.len() as u32);
        }
        kResultOk
    }
}
pub fn raw(bytes: Vec<u8>) -> Input {
    Input {
        id: Some(CString::new("dandrum.trigger.v1").unwrap()),
        attributes: Some(
            ComWrapper::new(Blob {
                bytes,
                reported: None,
                null_data: false,
                fail: false,
            })
            .to_com_ptr::<IAttributeList>()
            .unwrap(),
        ),
    }
}
pub fn packet(value: &serde_json::Value) -> ComPtr<IMessage> {
    ComWrapper::new(raw(serde_json::to_vec(value).unwrap()))
        .to_com_ptr::<IMessage>()
        .unwrap()
}
pub fn input(value: Input) -> ComPtr<IMessage> {
    ComWrapper::new(value).to_com_ptr::<IMessage>().unwrap()
}

pub unsafe fn decode(pointer: *mut IMessage) -> serde_json::Value {
    let message = unsafe { ComRef::from_raw(pointer) }.unwrap();
    let attributes = unsafe { ComRef::from_raw(message.getAttributes()) }.unwrap();
    let (mut data, mut size) = (std::ptr::null(), 0);
    assert_eq!(
        unsafe { attributes.getBinary(c"payload".as_ptr(), &mut data, &mut size) },
        kResultOk
    );
    assert!(!data.is_null() && size <= 2 * 1_048_576 + 4096);
    serde_json::from_slice(unsafe { std::slice::from_raw_parts(data.cast::<u8>(), size as usize) })
        .unwrap()
}
