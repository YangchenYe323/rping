use std::{ffi::CStr, time::Duration};

use clap::Parser;
use rping::Ping;

/// Simple program to greet a person
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Args {
  /// Name of the person to greet
  hostname: String,
}

fn to_cstring(s: &mut String) -> &CStr {
  s.push('\0');
  CStr::from_bytes_with_nul(s.as_bytes()).unwrap()
}

fn main() {
  let Args { mut hostname } = Args::parse();
  let hostname = to_cstring(&mut hostname);

  let mut p = Ping::new();
  p.add_host(hostname).unwrap();
  loop {
    assert_eq!(1, p.send().unwrap());
    let handle = p.iter().next().unwrap();
    let addr = handle.get_address();
    let seq = handle.get_sequence();
    let ttl = handle.get_received_ttl();
    let lat = handle.get_latency();
    println!(
      "From Host {}: icmp_seq={} ttl={} latency={} ms",
      addr, seq, ttl, lat
    );
    std::thread::sleep(Duration::from_millis(1000));
  }
}
