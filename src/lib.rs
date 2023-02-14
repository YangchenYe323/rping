#![feature(c_size_t)]
#![allow(non_upper_case_globals)]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![warn(missing_debug_implementations)]
#![allow(clippy::uninlined_format_args)]
#![allow(dead_code)] // todo(yangchen): remove these after work finished
#![allow(unused_imports)]

//! This crate provides a minimal-overhead type-safe Rust bindings for [liboping](https://noping.cc/)
//! It's expected to be dynamically linked to the library, and exposes the functionality for use in other
//! Rust projects.

#[allow(clippy::all)]
mod bindings;
mod oping;

pub use oping::*;
