use std::ffi::CStr;

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
#[derive(Debug)]
pub struct PingError<'a> {
  code: i32,
  // this points inside the Ping object.
  msg: &'a CStr,
}

impl<'a> PingError<'a> {
  fn new(code: i32, msg: &'a CStr) -> Self {
    Self { code, msg }
  }
}

impl<'a> core::fmt::Display for PingError<'a> {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    write!(f, "{:?}", self)
  }
}

impl<'a> std::error::Error for PingError<'a> {
  fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
    None
  }

  fn description(&self) -> &str {
    self.msg.to_str().unwrap()
  }

  fn cause(&self) -> Option<&dyn std::error::Error> {
    self.source()
  }
}

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

  /// Add a hosts to the current ping object so that it can be pinned simultaneously with
  /// all added hosts.
  ///
  /// * `self` - current [Ping] object
  /// * `host_name` - Host name of the target to ping
  pub fn add_host(&mut self, host_name: impl AsRef<CStr>) -> Result<()> {
    unsafe {
      let ret = ping_host_add(self.inner, host_name.as_ref().as_ptr());

      match ret {
        0 => Ok(()),
        _ => {
          let ptr = ping_get_error(self.inner);
          let c_str = CStr::from_ptr(ptr);
          Err(PingError::new(ret, c_str))
        }
      }
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

#[cfg(test)]
mod tests {
  use super::*;
  use byte_strings::c;

  #[test]
  fn it_works() {
    let mut p = Ping::new();
    p.add_host(c!("google.com")).unwrap();
  }
}
