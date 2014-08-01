// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Native os-thread management
//!
//! This modules contains bindings necessary for managing OS-level threads.
//! These functions operate outside of the rust runtime, creating threads
//! which are not used for scheduling in any way.

#![allow(non_camel_case_types)]

use core::prelude::*;

use alloc::boxed::Box;
use core::mem;
use core::uint;
use libc;

use platform;
use platform::stack;

/// This struct represents a native thread's state. This is used to join on an
/// existing thread created in the join-able state.
pub struct Thread<T> {
    native: platform::thread::rust_thread,
    joined: bool,
    packet: Box<Option<T>>,
}

static DEFAULT_STACK_SIZE: uint = 1024 * 1024;

// This is the starting point of rust os threads. The first thing we do
// is make sure that we don't trigger __morestack (also why this has a
// no_split_stack annotation), and then we extract the main function
// and invoke it.
#[no_split_stack]
extern fn thread_start(main: *mut libc::c_void) -> platform::thread::rust_thread_return {
    unsafe {
        stack::record_stack_bounds(0, uint::MAX);
        let f: Box<proc()> = mem::transmute(main);
        (*f)();
        mem::transmute(0 as platform::thread::rust_thread_return)
    }
}

// There are two impl blocks b/c if T were specified at the top then it's just a
// pain to specify a type parameter on Thread::spawn (which doesn't need the
// type parameter).
impl Thread<()> {

    /// Starts execution of a new OS thread.
    ///
    /// This function will not wait for the thread to join, but a handle to the
    /// thread will be returned.
    ///
    /// Note that the handle returned is used to acquire the return value of the
    /// procedure `main`. The `join` function will wait for the thread to finish
    /// and return the value that `main` generated.
    ///
    /// Also note that the `Thread` returned will *always* wait for the thread
    /// to finish executing. This means that even if `join` is not explicitly
    /// called, when the `Thread` falls out of scope its destructor will block
    /// waiting for the OS thread.
    pub fn start<T: Send>(main: proc():Send -> T) -> Thread<T> {
        Thread::start_stack(DEFAULT_STACK_SIZE, main)
    }

    /// Performs the same functionality as `start`, but specifies an explicit
    /// stack size for the new thread.
    pub fn start_stack<T: Send>(stack: uint, main: proc():Send -> T) -> Thread<T> {

        // We need the address of the packet to fill in to be stable so when
        // `main` fills it in it's still valid, so allocate an extra box to do
        // so.
        let packet = box None;
        let packet2: *mut Option<T> = unsafe {
            *mem::transmute::<&Box<Option<T>>, *const *mut Option<T>>(&packet)
        };
        let main = proc() unsafe { *packet2 = Some(main()); };
        let native = unsafe { platform::thread::create(stack, thread_start, box main) };

        Thread {
            native: native,
            joined: false,
            packet: packet,
        }
    }

    /// This will spawn a new thread, but it will not wait for the thread to
    /// finish, nor is it possible to wait for the thread to finish.
    ///
    /// This corresponds to creating threads in the 'detached' state on unix
    /// systems. Note that platforms may not keep the main program alive even if
    /// there are detached thread still running around.
    pub fn spawn(main: proc():Send) {
        Thread::spawn_stack(DEFAULT_STACK_SIZE, main)
    }

    /// Performs the same functionality as `spawn`, but explicitly specifies a
    /// stack size for the new thread.
    pub fn spawn_stack(stack: uint, main: proc():Send) {
        unsafe {
            let handle = platform::thread::create(stack, thread_start, box main);
            platform::thread::detach(handle);
        }
    }

    /// Relinquishes the CPU slot that this OS-thread is currently using,
    /// allowing another thread to run for awhile.
    pub fn yield_now() {
        unsafe { platform::thread::yield_now(); }
    }
}

impl<T: Send> Thread<T> {
    /// Wait for this thread to finish, returning the result of the thread's
    /// calculation.
    pub fn join(mut self) -> T {
        assert!(!self.joined);
        unsafe { platform::thread::join(self.native) };
        self.joined = true;
        assert!(self.packet.is_some());
        self.packet.take_unwrap()
    }
}

#[unsafe_destructor]
impl<T: Send> Drop for Thread<T> {
    fn drop(&mut self) {
        // This is required for correctness. If this is not done then the thread
        // would fill in a return box which no longer exists.
        if !self.joined {
            unsafe { platform::thread::join(self.native) };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Thread;

    #[test]
    fn smoke() { Thread::start(proc (){}).join(); }

    #[test]
    fn data() { assert_eq!(Thread::start(proc () { 1i }).join(), 1); }

    #[test]
    fn detached() { Thread::spawn(proc () {}) }

    #[test]
    fn small_stacks() {
        assert_eq!(42i, Thread::start_stack(0, proc () 42i).join());
        assert_eq!(42i, Thread::start_stack(1, proc () 42i).join());
    }
}
