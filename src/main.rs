enum PingObj {}

extern "C" {
  fn ping_construct() -> *mut PingObj;
}

fn main() {
  unsafe {
    ping_construct();
  }
}
