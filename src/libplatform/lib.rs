// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_name = "platform"]
#![license = "MIT/ASL2"]
#![crate_type = "rlib"]
#![doc(html_logo_url = "http://www.rust-lang.org/logos/rust-logo-128x128-blk-v2.png",
       html_favicon_url = "http://www.rust-lang.org/favicon.ico",
       html_root_url = "http://doc.rust-lang.org/master/")]

#![feature(macro_rules, phase, globs, thread_local, managed_boxes, asm)]
#![feature(linkage, lang_items, unsafe_destructor, default_type_params)]
#![no_std]
#![experimental]

#[phase(plugin, link)] extern crate core;
extern crate alloc;
extern crate libc;

#[cfg(test)] #[phase(plugin, link)] extern crate std;

pub mod mutex;
pub mod stack;
pub mod thread;
pub mod thread_local_storage;
pub mod time;

pub mod libunwind;
pub mod unwind;

pub mod flock;

#[cfg(windows)]
#[cfg(android)]
pub static OS_DEFAULT_STACK_ESTIMATE: uint = 1 << 20;
#[cfg(unix, not(android))]
pub static OS_DEFAULT_STACK_ESTIMATE: uint = 2 * (1 << 20);

#[cfg(windows)]
pub fn ignore_sigpipe() {}
#[cfg(unix)]
pub fn ignore_sigpipe() {
  use libc;
  use libc::funcs::posix01::signal::signal;
  unsafe {
      assert!(signal(libc::SIGPIPE, libc::SIG_IGN) != -1);
  }
}

/* from rustrt::local_heap */

#[cfg(unix)]
pub fn debug_mem() -> bool {
    // FIXME: Need to port the environment struct to newsched
    false
}

#[cfg(windows)]
pub fn debug_mem() -> bool {
    false
}

/* end from rustrt::local_heap */

#[cfg(not(test))]
mod std {
    pub use core::{fmt, option, cmp};
}
