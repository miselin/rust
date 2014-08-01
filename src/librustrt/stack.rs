// Copyright 2013 The Rust Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution and at
// http://rust-lang.org/COPYRIGHT.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

//! Rust stack-limit management
//!
//! Currently Rust uses a segmented-stack-like scheme in order to detect stack
//! overflow for rust tasks. In this scheme, the prologue of all functions are
//! preceded with a check to see whether the current stack limits are being
//! exceeded.
//!
//! This module provides the functionality necessary in order to manage these
//! stack limits (which are stored in platform-specific locations). The
//! functions here are used at the borders of the task lifetime in order to
//! manage these limits.
//!
//! This function is an unstable module because this scheme for stack overflow
//! detection is not guaranteed to continue in the future. Usage of this module
//! is discouraged unless absolutely necessary.

/// This function is invoked from rust's current __morestack function. Segmented
/// stacks are currently not enabled as segmented stacks, but rather one giant
/// stack segment. This means that whenever we run out of stack, we want to
/// truly consider it to be stack overflow rather than allocating a new stack.
#[cfg(not(test))] // in testing, use the original libstd's version
#[lang = "stack_exhausted"]
extern fn stack_exhausted() {
    use core::prelude::*;
    use alloc::boxed::Box;
    use local::Local;
    use task::Task;
    use core::intrinsics;
    use platform::stack::{RED_ZONE, get_sp_limit, record_sp_limit};

    unsafe {
        // We're calling this function because the stack just ran out. We need
        // to call some other rust functions, but if we invoke the functions
        // right now it'll just trigger this handler being called again. In
        // order to alleviate this, we move the stack limit to be inside of the
        // red zone that was allocated for exactly this reason.
        let limit = get_sp_limit();
        record_sp_limit(limit - RED_ZONE / 2);

        // This probably isn't the best course of action. Ideally one would want
        // to unwind the stack here instead of just aborting the entire process.
        // This is a tricky problem, however. There's a few things which need to
        // be considered:
        //
        //  1. We're here because of a stack overflow, yet unwinding will run
        //     destructors and hence arbitrary code. What if that code overflows
        //     the stack? One possibility is to use the above allocation of an
        //     extra 10k to hope that we don't hit the limit, and if we do then
        //     abort the whole program. Not the best, but kind of hard to deal
        //     with unless we want to switch stacks.
        //
        //  2. LLVM will optimize functions based on whether they can unwind or
        //     not. It will flag functions with 'nounwind' if it believes that
        //     the function cannot trigger unwinding, but if we do unwind on
        //     stack overflow then it means that we could unwind in any function
        //     anywhere. We would have to make sure that LLVM only places the
        //     nounwind flag on functions which don't call any other functions.
        //
        //  3. The function that overflowed may have owned arguments. These
        //     arguments need to have their destructors run, but we haven't even
        //     begun executing the function yet, so unwinding will not run the
        //     any landing pads for these functions. If this is ignored, then
        //     the arguments will just be leaked.
        //
        // Exactly what to do here is a very delicate topic, and is possibly
        // still up in the air for what exactly to do. Some relevant issues:
        //
        //  #3555 - out-of-stack failure leaks arguments
        //  #3695 - should there be a stack limit?
        //  #9855 - possible strategies which could be taken
        //  #9854 - unwinding on windows through __morestack has never worked
        //  #2361 - possible implementation of not using landing pads

        let task: Option<Box<Task>> = Local::try_take();
        let name = match task {
            Some(ref task) => {
                task.name.as_ref().map(|n| n.as_slice())
            }
            None => None
        };
        let name = name.unwrap_or("<unknown>");

        // See the message below for why this is not emitted to the
        // task's logger. This has the additional conundrum of the
        // logger may not be initialized just yet, meaning that an FFI
        // call would happen to initialized it (calling out to libuv),
        // and the FFI call needs 2MB of stack when we just ran out.
        rterrln!("task '{}' has overflowed its stack", name);

        intrinsics::abort();
    }
}
