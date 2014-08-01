// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_camel_case_types)]

pub use self::imp::*;

use libc;

pub type StartFn = extern "C" fn(*mut libc::c_void) -> self::imp::rust_thread_return;

#[cfg(windows)]
mod imp {
    use core::prelude::*;

    use alloc::boxed::Box;
    use core::cmp;
    use core::mem;
    use core::ptr;
    use libc;
    use libc::types::os::arch::extra::{LPSECURITY_ATTRIBUTES, SIZE_T, BOOL,
                                       LPVOID, DWORD, LPDWORD, HANDLE};
    use stack::RED_ZONE;

    pub type rust_thread = HANDLE;
    pub type rust_thread_return = DWORD;

    pub unsafe fn create(stack: uint, f: super::StartFn, p: Box<proc():Send>) -> rust_thread {
        let arg: *mut libc::c_void = mem::transmute(p);
        // FIXME On UNIX, we guard against stack sizes that are too small but
        // that's because pthreads enforces that stacks are at least
        // PTHREAD_STACK_MIN bytes big.  Windows has no such lower limit, it's
        // just that below a certain threshold you can't do anything useful.
        // That threshold is application and architecture-specific, however.
        // For now, the only requirement is that it's big enough to hold the
        // red zone.  Round up to the next 64 kB because that's what the NT
        // kernel does, might as well make it explicit.  With the current
        // 20 kB red zone, that makes for a 64 kB minimum stack.
        let stack_size = (cmp::max(stack, RED_ZONE) + 0xfffe) & (-0xfffe - 1);
        let ret = CreateThread(ptr::mut_null(), stack_size as libc::size_t,
                               f, arg, 0, ptr::mut_null());

        if ret as uint == 0 {
            // be sure to not leak the closure
            let _p: Box<proc():Send> = mem::transmute(arg);
            fail!("failed to spawn native thread: {}", ret);
        }
        return ret;
    }

    pub unsafe fn join(native: rust_thread) {
        use libc::consts::os::extra::INFINITE;
        WaitForSingleObject(native, INFINITE);
    }

    pub unsafe fn detach(native: rust_thread) {
        assert!(libc::CloseHandle(native) != 0);
    }

    pub unsafe fn yield_now() {
        // This function will return 0 if there are no other threads to execute,
        // but this also means that the yield was useless so this isn't really a
        // case that needs to be worried about.
        SwitchToThread();
    }

    #[allow(non_snake_case_functions)]
    extern "system" {
        fn CreateThread(lpThreadAttributes: LPSECURITY_ATTRIBUTES,
                        dwStackSize: SIZE_T,
                        lpStartAddress: super::StartFn,
                        lpParameter: LPVOID,
                        dwCreationFlags: DWORD,
                        lpThreadId: LPDWORD) -> HANDLE;
        fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;
        fn SwitchToThread() -> BOOL;
    }
}

#[cfg(unix)]
mod imp {
    use core::prelude::*;

    use alloc::boxed::Box;
    use core::cmp;
    use core::mem;
    use core::ptr;
    use libc::consts::os::posix01::{PTHREAD_CREATE_JOINABLE, PTHREAD_STACK_MIN};
    use libc;

    use stack::RED_ZONE;

    pub type rust_thread = libc::pthread_t;
    pub type rust_thread_return = *mut u8;

    pub unsafe fn create(stack: uint, f: super::StartFn, p: Box<proc():Send>) -> rust_thread {
        let mut native: libc::pthread_t = mem::zeroed();
        let mut attr: libc::pthread_attr_t = mem::zeroed();
        assert_eq!(pthread_attr_init(&mut attr), 0);
        assert_eq!(pthread_attr_setdetachstate(&mut attr,
                                               PTHREAD_CREATE_JOINABLE), 0);

        // Reserve room for the red zone, the runtime's stack of last resort.
        let stack_size = cmp::max(stack, RED_ZONE + min_stack_size(&attr) as uint);
        match pthread_attr_setstacksize(&mut attr, stack_size as libc::size_t) {
            0 => {
            },
            libc::EINVAL => {
                // EINVAL means |stack_size| is either too small or not a
                // multiple of the system page size.  Because it's definitely
                // >= PTHREAD_STACK_MIN, it must be an alignment issue.
                // Round up to the nearest page and try again.
                let page_size = libc::sysconf(libc::_SC_PAGESIZE) as uint;
                let stack_size = (stack_size + page_size - 1) &
                                 (-(page_size as int - 1) as uint - 1);
                assert_eq!(pthread_attr_setstacksize(&mut attr, stack_size as libc::size_t), 0);
            },
            errno => {
                // This cannot really happen.
                fail!("pthread_attr_setstacksize() error: {}", errno);
            },
        };

        let arg: *mut libc::c_void = mem::transmute(p);
        let ret = pthread_create(&mut native, &attr, f, arg);
        assert_eq!(pthread_attr_destroy(&mut attr), 0);

        if ret != 0 {
            // be sure to not leak the closure
            let _p: Box<proc():Send> = mem::transmute(arg);
            fail!("failed to spawn native thread: {}", ret);
        }
        native
    }

    pub unsafe fn join(native: rust_thread) {
        assert_eq!(pthread_join(native, ptr::mut_null()), 0);
    }

    pub unsafe fn detach(native: rust_thread) {
        assert_eq!(pthread_detach(native), 0);
    }

    pub unsafe fn yield_now() { assert_eq!(sched_yield(), 0); }
    // glibc >= 2.15 has a __pthread_get_minstack() function that returns
    // PTHREAD_STACK_MIN plus however many bytes are needed for thread-local
    // storage.  We need that information to avoid blowing up when a small stack
    // is created in an application with big thread-local storage requirements.
    // See #6233 for rationale and details.
    //
    // Link weakly to the symbol for compatibility with older versions of glibc.
    // Assumes that we've been dynamically linked to libpthread but that is
    // currently always the case.  Note that you need to check that the symbol
    // is non-null before calling it!
    #[cfg(target_os = "linux")]
    fn min_stack_size(attr: *const libc::pthread_attr_t) -> libc::size_t {
        type F = unsafe extern "C" fn(*const libc::pthread_attr_t) -> libc::size_t;
        extern {
            #[linkage = "extern_weak"]
            static __pthread_get_minstack: *const ();
        }
        if __pthread_get_minstack.is_null() {
            PTHREAD_STACK_MIN
        } else {
            unsafe { mem::transmute::<*const (), F>(__pthread_get_minstack)(attr) }
        }
    }

    // __pthread_get_minstack() is marked as weak but extern_weak linkage is
    // not supported on OS X, hence this kludge...
    #[cfg(not(target_os = "linux"))]
    fn min_stack_size(_: *const libc::pthread_attr_t) -> libc::size_t {
        PTHREAD_STACK_MIN
    }

    extern {
        fn pthread_create(native: *mut libc::pthread_t,
                          attr: *const libc::pthread_attr_t,
                          f: super::StartFn,
                          value: *mut libc::c_void) -> libc::c_int;
        fn pthread_join(native: libc::pthread_t,
                        value: *mut *mut libc::c_void) -> libc::c_int;
        fn pthread_attr_init(attr: *mut libc::pthread_attr_t) -> libc::c_int;
        fn pthread_attr_destroy(attr: *mut libc::pthread_attr_t) -> libc::c_int;
        fn pthread_attr_setstacksize(attr: *mut libc::pthread_attr_t,
                                     stack_size: libc::size_t) -> libc::c_int;
        fn pthread_attr_setdetachstate(attr: *mut libc::pthread_attr_t,
                                       state: libc::c_int) -> libc::c_int;
        fn pthread_detach(thread: libc::pthread_t) -> libc::c_int;
        fn sched_yield() -> libc::c_int;
    }
}
