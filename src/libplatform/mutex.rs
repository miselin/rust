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

#[cfg(unix)]
mod imp {
    use libc;
    use self::os::{PTHREAD_MUTEX_INITIALIZER, PTHREAD_COND_INITIALIZER,
                   pthread_mutex_t, pthread_cond_t};
    use core::cell::UnsafeCell;

    type pthread_mutexattr_t = libc::c_void;
    type pthread_condattr_t = libc::c_void;

    #[cfg(target_os = "freebsd")]
    mod os {
        use libc;

        pub type pthread_mutex_t = *mut libc::c_void;
        pub type pthread_cond_t = *mut libc::c_void;

        pub static PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t =
            0 as pthread_mutex_t;
        pub static PTHREAD_COND_INITIALIZER: pthread_cond_t =
            0 as pthread_cond_t;
    }

    #[cfg(target_os = "macos")]
    #[cfg(target_os = "ios")]
    mod os {
        use libc;

        #[cfg(target_arch = "x86_64")]
        static __PTHREAD_MUTEX_SIZE__: uint = 56;
        #[cfg(target_arch = "x86_64")]
        static __PTHREAD_COND_SIZE__: uint = 40;
        #[cfg(target_arch = "x86")]
        static __PTHREAD_MUTEX_SIZE__: uint = 40;
        #[cfg(target_arch = "x86")]
        static __PTHREAD_COND_SIZE__: uint = 24;
        #[cfg(target_arch = "arm")]
        static __PTHREAD_MUTEX_SIZE__: uint = 40;
        #[cfg(target_arch = "arm")]
        static __PTHREAD_COND_SIZE__: uint = 24;

        static _PTHREAD_MUTEX_SIG_init: libc::c_long = 0x32AAABA7;
        static _PTHREAD_COND_SIG_init: libc::c_long = 0x3CB0B1BB;

        #[repr(C)]
        pub struct pthread_mutex_t {
            __sig: libc::c_long,
            __opaque: [u8, ..__PTHREAD_MUTEX_SIZE__],
        }
        #[repr(C)]
        pub struct pthread_cond_t {
            __sig: libc::c_long,
            __opaque: [u8, ..__PTHREAD_COND_SIZE__],
        }

        pub static PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = pthread_mutex_t {
            __sig: _PTHREAD_MUTEX_SIG_init,
            __opaque: [0, ..__PTHREAD_MUTEX_SIZE__],
        };
        pub static PTHREAD_COND_INITIALIZER: pthread_cond_t = pthread_cond_t {
            __sig: _PTHREAD_COND_SIG_init,
            __opaque: [0, ..__PTHREAD_COND_SIZE__],
        };
    }

    #[cfg(target_os = "linux")]
    mod os {
        use libc;

        // minus 8 because we have an 'align' field
        #[cfg(target_arch = "x86_64")]
        static __SIZEOF_PTHREAD_MUTEX_T: uint = 40 - 8;
        #[cfg(target_arch = "x86")]
        static __SIZEOF_PTHREAD_MUTEX_T: uint = 24 - 8;
        #[cfg(target_arch = "arm")]
        static __SIZEOF_PTHREAD_MUTEX_T: uint = 24 - 8;
        #[cfg(target_arch = "mips")]
        static __SIZEOF_PTHREAD_MUTEX_T: uint = 24 - 8;
        #[cfg(target_arch = "mipsel")]
        static __SIZEOF_PTHREAD_MUTEX_T: uint = 24 - 8;
        #[cfg(target_arch = "x86_64")]
        static __SIZEOF_PTHREAD_COND_T: uint = 48 - 8;
        #[cfg(target_arch = "x86")]
        static __SIZEOF_PTHREAD_COND_T: uint = 48 - 8;
        #[cfg(target_arch = "arm")]
        static __SIZEOF_PTHREAD_COND_T: uint = 48 - 8;
        #[cfg(target_arch = "mips")]
        static __SIZEOF_PTHREAD_COND_T: uint = 48 - 8;
        #[cfg(target_arch = "mipsel")]
        static __SIZEOF_PTHREAD_COND_T: uint = 48 - 8;

        #[repr(C)]
        pub struct pthread_mutex_t {
            __align: libc::c_longlong,
            size: [u8, ..__SIZEOF_PTHREAD_MUTEX_T],
        }
        #[repr(C)]
        pub struct pthread_cond_t {
            __align: libc::c_longlong,
            size: [u8, ..__SIZEOF_PTHREAD_COND_T],
        }

        pub static PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = pthread_mutex_t {
            __align: 0,
            size: [0, ..__SIZEOF_PTHREAD_MUTEX_T],
        };
        pub static PTHREAD_COND_INITIALIZER: pthread_cond_t = pthread_cond_t {
            __align: 0,
            size: [0, ..__SIZEOF_PTHREAD_COND_T],
        };
    }
    #[cfg(target_os = "android")]
    mod os {
        use libc;

        #[repr(C)]
        pub struct pthread_mutex_t { value: libc::c_int }
        #[repr(C)]
        pub struct pthread_cond_t { value: libc::c_int }

        pub static PTHREAD_MUTEX_INITIALIZER: pthread_mutex_t = pthread_mutex_t {
            value: 0,
        };
        pub static PTHREAD_COND_INITIALIZER: pthread_cond_t = pthread_cond_t {
            value: 0,
        };
    }

    pub struct Mutex {
        lock: UnsafeCell<pthread_mutex_t>,
        cond: UnsafeCell<pthread_cond_t>,
    }

    pub static MUTEX_INIT: Mutex = Mutex {
        lock: UnsafeCell { value: PTHREAD_MUTEX_INITIALIZER },
        cond: UnsafeCell { value: PTHREAD_COND_INITIALIZER },
    };

    impl Mutex {
        pub unsafe fn new() -> Mutex {
            // As mutex might be moved and address is changing it
            // is better to avoid initialization of potentially
            // opaque OS data before it landed
            let m = Mutex {
                lock: UnsafeCell::new(PTHREAD_MUTEX_INITIALIZER),
                cond: UnsafeCell::new(PTHREAD_COND_INITIALIZER),
            };

            return m;
        }

        pub unsafe fn lock(&self) { pthread_mutex_lock(self.lock.get()); }
        pub unsafe fn unlock(&self) { pthread_mutex_unlock(self.lock.get()); }
        pub unsafe fn signal(&self) { pthread_cond_signal(self.cond.get()); }
        pub unsafe fn wait(&self) {
            pthread_cond_wait(self.cond.get(), self.lock.get());
        }
        pub unsafe fn trylock(&self) -> bool {
            pthread_mutex_trylock(self.lock.get()) == 0
        }
        pub unsafe fn destroy(&self) {
            pthread_mutex_destroy(self.lock.get());
            pthread_cond_destroy(self.cond.get());
        }
    }

    extern {
        fn pthread_mutex_destroy(lock: *mut pthread_mutex_t) -> libc::c_int;
        fn pthread_cond_destroy(cond: *mut pthread_cond_t) -> libc::c_int;
        fn pthread_mutex_lock(lock: *mut pthread_mutex_t) -> libc::c_int;
        fn pthread_mutex_trylock(lock: *mut pthread_mutex_t) -> libc::c_int;
        fn pthread_mutex_unlock(lock: *mut pthread_mutex_t) -> libc::c_int;

        fn pthread_cond_wait(cond: *mut pthread_cond_t,
                             lock: *mut pthread_mutex_t) -> libc::c_int;
        fn pthread_cond_signal(cond: *mut pthread_cond_t) -> libc::c_int;
    }
}

#[cfg(windows)]
mod imp {
    use core::atomics;
    use core::ptr;
    use core::intrinsics;
    use libc::{HANDLE, BOOL, LPSECURITY_ATTRIBUTES, c_void, DWORD, LPCSTR};
    use libc;

    type LPCRITICAL_SECTION = *mut c_void;
    static SPIN_COUNT: DWORD = 4000;
    #[cfg(target_arch = "x86")]
    static CRIT_SECTION_SIZE: uint = 24;
    #[cfg(target_arch = "x86_64")]
    static CRIT_SECTION_SIZE: uint = 40;

    pub struct Mutex {
        // pointers for the lock/cond handles, atomically updated
        lock: atomics::AtomicUint,
        cond: atomics::AtomicUint,
    }

    pub static MUTEX_INIT: Mutex = Mutex {
        lock: atomics::INIT_ATOMIC_UINT,
        cond: atomics::INIT_ATOMIC_UINT,
    };

    impl Mutex {
        pub unsafe fn new() -> Mutex {
            Mutex {
                lock: atomics::AtomicUint::new(init_lock()),
                cond: atomics::AtomicUint::new(init_cond()),
            }
        }
        pub unsafe fn lock(&self) {
            EnterCriticalSection(self.getlock() as LPCRITICAL_SECTION)
        }
        pub unsafe fn trylock(&self) -> bool {
            TryEnterCriticalSection(self.getlock() as LPCRITICAL_SECTION) != 0
        }
        pub unsafe fn unlock(&self) {
            LeaveCriticalSection(self.getlock() as LPCRITICAL_SECTION)
        }

        pub unsafe fn wait(&self) {
            self.unlock();
            WaitForSingleObject(self.getcond() as HANDLE, libc::INFINITE);
            self.lock();
        }

        pub unsafe fn signal(&self) {
            assert!(SetEvent(self.getcond() as HANDLE) != 0);
        }

        /// This function is especially unsafe because there are no guarantees made
        /// that no other thread is currently holding the lock or waiting on the
        /// condition variable contained inside.
        pub unsafe fn destroy(&self) {
            let lock = self.lock.swap(0, atomics::SeqCst);
            let cond = self.cond.swap(0, atomics::SeqCst);
            if lock != 0 { free_lock(lock) }
            if cond != 0 { free_cond(cond) }
        }

        unsafe fn getlock(&self) -> *mut c_void {
            match self.lock.load(atomics::SeqCst) {
                0 => {}
                n => return n as *mut c_void
            }
            let lock = init_lock();
            match self.lock.compare_and_swap(0, lock, atomics::SeqCst) {
                0 => return lock as *mut c_void,
                _ => {}
            }
            free_lock(lock);
            return self.lock.load(atomics::SeqCst) as *mut c_void;
        }

        unsafe fn getcond(&self) -> *mut c_void {
            match self.cond.load(atomics::SeqCst) {
                0 => {}
                n => return n as *mut c_void
            }
            let cond = init_cond();
            match self.cond.compare_and_swap(0, cond, atomics::SeqCst) {
                0 => return cond as *mut c_void,
                _ => {}
            }
            free_cond(cond);
            return self.cond.load(atomics::SeqCst) as *mut c_void;
        }
    }

    pub unsafe fn init_lock() -> uint {
        let block = libc::malloc(CRIT_SECTION_SIZE as uint) as *mut c_void;
        if block.is_null() {
            ::oom();
        }
        InitializeCriticalSectionAndSpinCount(block, SPIN_COUNT);
        return block as uint;
    }

    pub unsafe fn init_cond() -> uint {
        return CreateEventA(ptr::mut_null(), libc::FALSE, libc::FALSE,
                            ptr::null()) as uint;
    }

    pub unsafe fn free_lock(h: uint) {
        DeleteCriticalSection(h as LPCRITICAL_SECTION);
        libc::free(h as *mut c_void);
    }

    pub unsafe fn free_cond(h: uint) {
        let block = h as HANDLE;
        libc::CloseHandle(block);
    }

    #[allow(non_snake_case_functions)]
    extern "system" {
        fn CreateEventA(lpSecurityAttributes: LPSECURITY_ATTRIBUTES,
                        bManualReset: BOOL,
                        bInitialState: BOOL,
                        lpName: LPCSTR) -> HANDLE;
        fn InitializeCriticalSectionAndSpinCount(
                        lpCriticalSection: LPCRITICAL_SECTION,
                        dwSpinCount: DWORD) -> BOOL;
        fn DeleteCriticalSection(lpCriticalSection: LPCRITICAL_SECTION);
        fn EnterCriticalSection(lpCriticalSection: LPCRITICAL_SECTION);
        fn LeaveCriticalSection(lpCriticalSection: LPCRITICAL_SECTION);
        fn TryEnterCriticalSection(lpCriticalSection: LPCRITICAL_SECTION) -> BOOL;
        fn SetEvent(hEvent: HANDLE) -> BOOL;
        fn WaitForSingleObject(hHandle: HANDLE, dwMilliseconds: DWORD) -> DWORD;
    }
}
