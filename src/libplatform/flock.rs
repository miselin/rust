// Copyright 2014 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![allow(non_camel_case_types)]

#[cfg(target_os = "linux")]
pub mod os {
    use libc;

    pub struct flock {
        pub l_type: libc::c_short,
        pub l_whence: libc::c_short,
        pub l_start: libc::off_t,
        pub l_len: libc::off_t,
        pub l_pid: libc::pid_t,

        // not actually here, but brings in line with freebsd
        pub l_sysid: libc::c_int,
    }

    pub static F_WRLCK: libc::c_short = 1;
    pub static F_UNLCK: libc::c_short = 2;
    pub static F_SETLK: libc::c_int = 6;
    pub static F_SETLKW: libc::c_int = 7;
}

#[cfg(target_os = "freebsd")]
pub mod os {
    use libc;

    pub struct flock {
        pub l_start: libc::off_t,
        pub l_len: libc::off_t,
        pub l_pid: libc::pid_t,
        pub l_type: libc::c_short,
        pub l_whence: libc::c_short,
        pub l_sysid: libc::c_int,
    }

    pub static F_UNLCK: libc::c_short = 2;
    pub static F_WRLCK: libc::c_short = 3;
    pub static F_SETLK: libc::c_int = 12;
    pub static F_SETLKW: libc::c_int = 13;
}

#[cfg(target_os = "macos")]
#[cfg(target_os = "ios")]
pub mod os {
    use libc;

    pub struct flock {
        pub l_start: libc::off_t,
        pub l_len: libc::off_t,
        pub l_pid: libc::pid_t,
        pub l_type: libc::c_short,
        pub l_whence: libc::c_short,

        // not actually here, but brings in line with freebsd
        pub l_sysid: libc::c_int,
    }

    pub static F_UNLCK: libc::c_short = 2;
    pub static F_WRLCK: libc::c_short = 3;
    pub static F_SETLK: libc::c_int = 8;
    pub static F_SETLKW: libc::c_int = 9;
}
