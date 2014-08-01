// Copyright 2013-2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

use libc;

#[cfg(unix, not(target_os = "macos"), not(target_os = "ios"))]
mod imp {
    use libc::{c_int, timespec};

    // Apparently android provides this in some other library?
    #[cfg(not(target_os = "android"))]
    #[link(name = "rt")]
    extern {}

    extern {
        pub fn clock_gettime(clk_id: c_int, tp: *mut timespec) -> c_int;
    }

}
#[cfg(target_os = "macos")]
#[cfg(target_os = "ios")]
mod imp {
    use libc::{timeval, timezone, c_int, mach_timebase_info};

    extern {
        pub fn gettimeofday(tp: *mut timeval, tzp: *mut timezone) -> c_int;
        pub fn mach_absolute_time() -> u64;
        pub fn mach_timebase_info(info: *mut mach_timebase_info) -> c_int;
    }
}

#[cfg(windows)]
pub unsafe fn os_get_time() -> (i64, i32) {
    static NANOSECONDS_FROM_1601_TO_1970: u64 = 11644473600000000;

    let mut time = libc::FILETIME {
        dwLowDateTime: 0,
        dwHighDateTime: 0,
    };
    libc::GetSystemTimeAsFileTime(&mut time);

    // A FILETIME contains a 64-bit value representing the number of
    // hectonanosecond (100-nanosecond) intervals since 1601-01-01T00:00:00Z.
    // http://support.microsoft.com/kb/167296/en-us
    let ns_since_1601 = ((time.dwHighDateTime as u64 << 32) |
                         (time.dwLowDateTime  as u64 <<  0)) / 10;
    let ns_since_1970 = ns_since_1601 - NANOSECONDS_FROM_1601_TO_1970;

    ((ns_since_1970 / 1000000) as i64,
     ((ns_since_1970 % 1000000) * 1000) as i32)
}

#[cfg(target_os = "macos")]
#[cfg(target_os = "ios")]
pub unsafe fn os_get_time() -> (i64, i32) {
    use std::ptr;
    let mut tv = libc::timeval { tv_sec: 0, tv_usec: 0 };
    imp::gettimeofday(&mut tv, ptr::mut_null());
    (tv.tv_sec as i64, tv.tv_usec * 1000)
}

#[cfg(not(target_os = "macos"), not(target_os = "ios"), not(windows))]
pub unsafe fn os_get_time() -> (i64, i32) {
    let mut tv = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    imp::clock_gettime(libc::CLOCK_REALTIME, &mut tv);
    (tv.tv_sec as i64, tv.tv_nsec as i32)
}

#[cfg(windows)]
pub fn os_precise_time_ns() -> u64 {
    let mut ticks_per_s = 0;
    assert_eq!(unsafe {
        libc::QueryPerformanceFrequency(&mut ticks_per_s)
    }, 1);
    let ticks_per_s = if ticks_per_s == 0 {1} else {ticks_per_s};
    let mut ticks = 0;
    assert_eq!(unsafe {
        libc::QueryPerformanceCounter(&mut ticks)
    }, 1);

    return (ticks as u64 * 1000000000) / (ticks_per_s as u64);
}

#[cfg(target_os = "macos")]
#[cfg(target_os = "ios")]
pub fn os_precise_time_ns() -> u64 {
    static mut TIMEBASE: libc::mach_timebase_info = libc::mach_timebase_info { numer: 0,
                                                                               denom: 0 };
    static mut ONCE: std::sync::Once = std::sync::ONCE_INIT;
    unsafe {
        ONCE.doit(|| {
            imp::mach_timebase_info(&mut TIMEBASE);
        });
        let time = imp::mach_absolute_time();
        time * TIMEBASE.numer as u64 / TIMEBASE.denom as u64
    }
}

#[cfg(not(windows), not(target_os = "macos"), not(target_os = "ios"))]
pub fn os_precise_time_ns() -> u64 {
    let mut ts = libc::timespec { tv_sec: 0, tv_nsec: 0 };
    unsafe {
        imp::clock_gettime(libc::CLOCK_MONOTONIC, &mut ts);
    }
    return (ts.tv_sec as u64) * 1000000000 + (ts.tv_nsec as u64)
}
