use core::ffi::c_size_t;
use std::{
  ffi::{c_void, CStr},
  marker::PhantomData,
  mem::MaybeUninit,
};

use libc::{IP_TOS, NI_MAXHOST};

use crate::bindings::*;

type Result<'a, T> = core::result::Result<T, PingError<'a>>;

/// Error Type of our Ping structure.
/// The lifetime is tied to the lifetime of the Ping object
/// because the error references a C-String stored in the object. And
/// multiple errors are written to the same buffer address,
/// reference: https://github.com/octo/liboping/blob/master/src/liboping.c
/// Therefore construction of a second [PingError] invalidates the first one becaue
/// the error msg might no longer be valid.
///
/// ## Example
/// ```compile_fail
/// use rping::Ping;
/// use std::ffi::CStr;
/// let mut p = Ping::new();
/// let s1: &'static CStr = unsafe {
///   std::mem::transmute("aaaaunjojlk.com")
/// };
/// let s2: &'static CStr = unsafe {
///   std::mem::transmute("src.com")
/// };
/// let r1 = p.add_host(s1);
/// let r2 = p.add_host(s2); // Error! p is mutably borrowed by r1
///
/// println!("{:?}", r1);
/// ```
///
/// Due to this fact, [PingError] doesn't play well with RUst's ? operator if it is
/// generated by a local [Ping] object, and users are expected to implement their own error
/// handling on top of it.
///
/// ## Example
/// ```compile_fail
/// fn main() -> Result<(), Box<dyn Error>> {
///   use rping::Ping;
///   use std::ffi::CStr;
///   let mut p = Ping::new();
///   let s: &'static CStr = unsafe {
///     std::mem::transmute("github.com");
///   }
///   p.add_host(s)?; // Error! The PingError cannot be propogated outside of the function after p gets dropped.
///   Ok(())
/// }
#[derive(Debug)]
pub struct PingError<'a> {
  // this points inside the Ping object.
  msg: &'a CStr,
}

impl<'a> PingError<'a> {
  fn new(msg: &'a CStr) -> Self {
    Self { msg }
  }
}

impl<'a> core::fmt::Display for PingError<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

// impl<'a> std::error::Error for PingError<'a> {
//   fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
//     None
//   }

//   fn description(&self) -> &str {
//     self.msg.to_str().unwrap()
//   }

//   fn cause(&self) -> Option<&dyn std::error::Error> {
//     self.source()
//   }
// }

/// Safe Rust Wrappers around `pingobj_t` in [liboping](https://noping.cc/)
#[derive(Debug)]
pub struct Ping {
  inner: *mut pingobj_t,
}

impl Ping {
  /// Create a new [Ping].
  pub fn new() -> Self {
    // SAFETY: ping_construct never fails and always
    // returns a valid pointer.
    unsafe {
      Self {
        inner: ping_construct(),
      }
    }
  }

  unsafe fn map_err(&mut self, ret: i32) -> Result<()> {
    if ret >= 0 {
      Ok(())
    } else {
      let ptr = ping_get_error(self.inner);
      let c_str = CStr::from_ptr(ptr);
      Err(PingError::new(c_str))
    }
  }

  /// Add a host to the current ping object so that it can be pinged simultaneously with
  /// all added hosts.
  ///
  pub fn add_host(&mut self, host_name: impl AsRef<CStr>) -> Result<()> {
    unsafe {
      let ret = ping_host_add(self.inner, host_name.as_ref().as_ptr());
      self.map_err(ret)
    }
  }

  /// Remove a host from the lists to be pinged.
  /// Returns error if the host is not resolved or not found.
  ///
  pub fn remove_host(&mut self, host_name: impl AsRef<CStr>) -> Result<()> {
    unsafe {
      let ret = ping_host_remove(self.inner, host_name.as_ref().as_ptr());
      self.map_err(ret)
    }
  }

  /// Returns a [PingIter] object for iterating over all the associated host
  /// and get information.
  ///
  pub fn iter(&self) -> PingIter<'_> {
    PingIter {
      inner: unsafe { ping_iterator_get(self.inner) },
      _phantom: Default::default(),
    }
  }

  /// Send ICMP echo messages to all added host associated with self and block
  /// waiting for responses until timeout.
  /// Return the number of received echo messages on success.
  pub fn send(&mut self) -> Result<i32> {
    unsafe {
      let ret = ping_send(self.inner);
      self.map_err(ret)?;
      Ok(ret)
    }
  }
}

impl Default for Ping {
  fn default() -> Self {
    Self::new()
  }
}

impl Drop for Ping {
  fn drop(&mut self) {
    // SAFETY: self.inner is returned by a valid call of
    // ping_construct, and cannot be modified or invalidated
    // during its lifetime.
    unsafe {
      ping_destroy(self.inner);
    }
  }
}

/// Immutable Iterator Type for [Ping],
#[derive(Debug)]
pub struct PingIter<'a> {
  inner: *mut pingobj_iter_t,
  _phantom: PhantomData<&'a ()>,
}

#[derive(Debug)]
pub struct IterInfoHandle<'a> {
  inner: *mut pingobj_iter_t,
  _phantom: PhantomData<&'a ()>,
}

impl<'a> Iterator for PingIter<'a> {
  type Item = IterInfoHandle<'a>;

  fn next(&mut self) -> Option<Self::Item> {
    unsafe {
      let n = ping_iterator_next(self.inner);
      if self.inner.is_null() {
        None
      } else {
        let handle = Some(IterInfoHandle {
          inner: self.inner,
          _phantom: Default::default(),
        });
        self.inner = n;
        handle
      }
    }
  }
}

const INFO_BUFFER_SIZE: usize = 256;

impl<'a> IterInfoHandle<'a> {
  /// Get a hostname from the handle stored as a string.
  unsafe fn get_info_string(&self, info: i32, size_hint: usize) -> String {
    // buf will hold data read from liboping
    let mut vec = Vec::with_capacity(size_hint);
    vec.resize(size_hint, MaybeUninit::<u8>::uninit());
    let buf = vec.into_boxed_slice();

    // buf_len will hold actually read data size or necessary data size of
    // buf is not long enough
    let buf_len: u64 = size_hint as u64;

    let ret = ping_iterator_get_info(
      self.inner,
      info,
      buf.as_ptr() as *const c_void as *mut c_void,
      (&buf_len) as *const u64 as *mut u64,
    );

    // Shouldn't return error
    debug_assert_ne!(libc::EINVAL, ret);

    // todo(yangchen): Reallocate a larger buffer?
    assert!(
      buf_len <= size_hint as u64,
      "Buffer Too Short, Needed {} Bytes",
      buf_len
    );

    // Assemble the received bytes into a string.
    let buf = Box::leak(buf);
    let str = buf.as_ptr() as *mut u8;
    let str_len = buf_len - 1; // Omit the trailing null

    // SAFETY:
    // 1. str comes from buf, which is allocated by rust allocator.
    // 2. capacity is correct as we allocated it explicitly.
    // 3. the first str_len bytes are guaranteed to be valid utf-8 because hostnames can only
    // contain valid utf-8 characters: https://www.rfc-editor.org/rfc/rfc952
    // Invalid hostnames are guaranteed to be rejected by `add_host`
    String::from_raw_parts(str, str_len as usize, INFO_BUFFER_SIZE)
  }

  unsafe fn get_info_double(&self, info: i32) -> f64 {
    let buf: f64 = 0.0;
    let buf_len: u64 = 64;
    let ret = ping_iterator_get_info(
      self.inner,
      info,
      (&buf) as *const f64 as *const c_void as *mut c_void,
      (&buf_len) as *const u64 as *mut u64,
    );

    assert_ne!(libc::EINVAL, ret);
    buf
  }

  unsafe fn get_info_int(&self, info: i32) -> i32 {
    let buf: i32 = 0;
    let buf_len: u64 = 32;
    let ret = ping_iterator_get_info(
      self.inner,
      info,
      (&buf) as *const i32 as *const c_void as *mut c_void,
      (&buf_len) as *const u64 as *mut u64,
    );

    assert_ne!(libc::EINVAL, ret);
    buf
  }

  /// Get the user-supplied hostname associated with this host.
  /// This is guaranteed to be equal to user-supplied argument to
  /// `Ping::add_host` without the trailing \0.
  ///
  pub fn get_hostname_user(&self) -> String {
    unsafe { self.get_info_string(PING_INFO_USERNAME as i32, libc::NI_MAXHOST as usize) }
  }

  /// Get the system-parsed hostname associated with the host.
  /// this might not equal the user-supplied name and is looked up
  /// every time this function is called.
  ///
  pub fn get_hostname(&self) -> String {
    unsafe { self.get_info_string(PING_INFO_HOSTNAME as i32, libc::NI_MAXHOST as usize) }
  }

  /// Get the IP address in ASCII format of the associated host.
  ///
  pub fn get_address(&self) -> String {
    unsafe { self.get_info_string(PING_INFO_ADDRESS as i32, 40) }
  }

  /// Get the last measured latency of receiving an echo response from
  /// the associated host measured in milliseconds.
  /// Result is negative if timeout occured in between.
  ///
  pub fn get_latency(&self) -> f64 {
    unsafe { self.get_info_double(PING_INFO_LATENCY as i32) }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use byte_strings::c;

  #[test]
  fn it_works() {
    let mut p = Ping::new();
    p.add_host(c!("google.com")).unwrap();
  }

  #[test]
  fn reject_invalid_encoding() {
    let mut p = Ping::new();
    // Invalid encodings for hostname should be rejected
    let host: [i8; 4] = [b'a' as i8, -2, -3, 0];
    let cstr = unsafe { CStr::from_ptr(host.as_ptr() as *const i8) };
    let res = p.add_host(cstr);
    assert!(res.is_err());
  }

  #[test]
  fn err_remove_nonexistent() {
    let mut p = Ping::new();
    let host = c!("google.com");
    assert!(p.remove_host(host).is_err());

    p.add_host(host).unwrap();
    p.remove_host(host).unwrap();
  }

  #[test]
  fn test_send() {
    let mut p = Ping::new();
    let host = c!("google.com");
    p.add_host(host).unwrap();
    assert_eq!(1, p.send().unwrap());

    let mut iter = p.iter();
    println!("{:?}", iter);
    let handle = iter.next().unwrap();
    let c = handle.get_hostname_user();
    assert_eq!("google.com", c);
  }

  #[test]
  fn test_address() {
    let mut p = Ping::new();
    let host = c!("localhost");
    p.add_host(host).unwrap();
    assert_eq!(1, p.send().unwrap());

    let mut iter = p.iter();
    let handle = iter.next().unwrap();
    let c = handle.get_address();
    assert!(c.starts_with("127.0.0.1"));
  }
}
