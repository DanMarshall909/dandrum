//! Borrowed SDK I/O; native state framing and admission stay in trigger-host.
use std::{
    ffi::c_void,
    io::{self, Read, Write},
};
use vst3::{
    ComRef,
    Steinberg::{IBStream, IBStreamTrait, kResultOk},
};
pub(crate) struct Stream<'a>(ComRef<'a, IBStream>);
impl<'a> Stream<'a> {
    /// Caller retains a valid standard stream through the complete foreign call.
    pub unsafe fn from_raw(raw: *mut IBStream) -> Option<Self> {
        unsafe { ComRef::from_raw(raw) }.map(Self)
    }
    fn transfer(&self, buffer: *mut c_void, length: usize, writing: bool) -> io::Result<usize> {
        if length == 0 {
            return Ok(0);
        }
        let requested = length.min(i32::MAX as usize) as i32;
        let mut actual = 0;
        let result = unsafe {
            if writing {
                self.0.write(buffer, requested, &mut actual)
            } else {
                self.0.read(buffer, requested, &mut actual)
            }
        };
        if result != kResultOk || actual < 0 || actual > requested {
            return Err(io::Error::other("invalid or failed VST3 stream progress"));
        }
        if writing && actual == 0 {
            return Err(io::ErrorKind::WriteZero.into());
        }
        Ok(actual as usize)
    }
}
impl Read for Stream<'_> {
    fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
        self.transfer(buffer.as_mut_ptr().cast(), buffer.len(), false)
    }
}
impl Write for Stream<'_> {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        self.transfer(buffer.as_ptr().cast_mut().cast(), buffer.len(), true)
    }
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

#[cfg(test)]
#[path = "../tests/common/stream.rs"]
mod host_fixture;
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn sdk_progress_distinguishes_short_io_eof_failure_and_invalid_counts() {
        let (observation, host) = host_fixture::stream(vec![1, 2, 3], 2);
        let mut stream = unsafe { Stream::from_raw(host.as_ptr()) }.unwrap();
        let mut bytes = [0; 4];
        assert_eq!(stream.read(&mut bytes).unwrap(), 2);
        assert_eq!(&bytes[..2], &[1, 2]);
        assert_eq!(stream.read(&mut bytes).unwrap(), 1);
        assert_eq!(bytes[0], 3);
        assert_eq!(stream.read(&mut bytes).unwrap(), 0);
        assert_eq!(stream.write(&[4, 5, 6]).unwrap(), 2);
        observation.maximum.set(0);
        assert_eq!(
            stream.write(&[1]).unwrap_err().kind(),
            io::ErrorKind::WriteZero
        );
        observation.maximum.set(2);
        observation.fail.set(true);
        assert!(stream.read(&mut bytes).is_err());
        assert!(stream.write(&[1]).is_err());
        observation.fail.set(false);
        for count in [
            host_fixture::ReplyCount::Negative,
            host_fixture::ReplyCount::Excess,
        ] {
            observation.reply_count.set(count);
            assert!(stream.read(&mut bytes).is_err());
            assert!(stream.write(&[1]).is_err());
        }
    }
    #[test]
    fn empty_sdk_io_is_successful_without_any_foreign_operation() {
        let (observation, host) = host_fixture::stream(vec![], 1);
        observation.limit.set(0);
        let mut stream = unsafe { Stream::from_raw(host.as_ptr()) }.unwrap();
        assert_eq!(stream.read(&mut []).unwrap(), 0);
        assert_eq!(stream.write(&[]).unwrap(), 0);
        stream.flush().unwrap();
        assert_eq!(observation.calls.get(), 0);
    }
}
