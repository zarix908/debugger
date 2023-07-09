mod context;

use std::{ffi::CStr, ptr};

use context::Context;

#[no_mangle]
pub extern "C" fn load(program_pid: i32, program_path: *const libc::c_char) -> *const libc::c_void {
    // SAFETY: The caller must guarantee that pointer is valid.
    let path = match unsafe { CStr::from_ptr(program_path).to_str() } {
        Ok(v) => v,
        Err(_) => return ptr::null(),
    };

    let debugger = match mdbg_rs::load_in_memory(program_pid, path) {
        Ok(debugger) => debugger,
        Err(_) => return ptr::null(),
    };

    Context::store(debugger) as *const libc::c_void
}

#[no_mangle]
pub extern "C" fn wait_attach(ctx: *const libc::c_void) -> i64 {
    Context::from(ctx as u64)
        .and_then(|mut ctx| ctx.with_debugger(|d| d.wait_attach().or(Err(()))))
        .and(Ok(0))
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn get_load_addr(program_pid: i32, program_path: *const libc::c_char) -> u64 {
    // SAFETY: The caller must guarantee that pointer is valid.
    let path = match unsafe { CStr::from_ptr(program_path).to_str() } {
        Ok(v) => v,
        Err(_) => return 0,
    };

    match mdbg_rs::linux_maps::get_load_addr(program_pid, path) {
        Ok(addr) => addr,
        Err(_) => 0,
    }
}

#[no_mangle]
pub extern "C" fn set_load_addr(ctx: *const libc::c_void, addr: u64) -> i64 {
    Context::from(ctx as u64)
        .and_then(|mut ctx| ctx.with_debugger(|d| Ok(d.set_load_addr(addr))))
        .and(Ok(0))
        .unwrap_or(-1)
}

#[repr(C)]
pub struct StatusResult {
    exited: bool,
    status: i32,
    err: i64,
}

#[no_mangle]
pub extern "C" fn continue_execution(ctx: *const libc::c_void) -> StatusResult {
    Context::from(ctx as u64)
        .and_then(|mut ctx| {
            ctx.with_debugger(|d| {
                let status = d.continue_execution().or(Err(()))?;
                Ok(StatusResult {
                    exited: status.is_some(),
                    status: status.unwrap_or(0),
                    err: 0,
                })
            })
        })
        .unwrap_or(StatusResult {
            exited: false,
            status: 0,
            err: -1,
        })
}

#[no_mangle]
pub extern "C" fn set_breakpoint(
    ctx: *const libc::c_void,
    filename: *const libc::c_char,
    line: u64,
) -> i64 {
    // SAFETY: The caller must guarantee that pointer is valid.
    let filename = match unsafe { CStr::from_ptr(filename).to_str() } {
        Ok(v) => v.to_owned(),
        Err(_) => return -1,
    };

    let breakpoint_ref = mdbg_rs::BreakpointRef::Line { filename, line };
    Context::from(ctx as u64)
        .and_then(|mut ctx| ctx.with_debugger(|d| Ok(d.set_breakpoint(breakpoint_ref.clone()))))
        .and(Ok(0))
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn get_register_value(
    ctx: *const libc::c_void,
    register: *const libc::c_char,
    value: *mut u64,
) -> i64 {
    // SAFETY: The caller must guarantee that pointer is valid.
    let reg = match unsafe { CStr::from_ptr(register).to_str() } {
        Ok(v) => v,
        Err(_) => return -1,
    };

    let reg_selector = &mdbg_rs::RegSelector::Name(reg);
    Context::from(ctx as u64)
        .and_then(|mut ctx| ctx.with_debugger(|d| d.get_register_value(reg_selector).or(Err(()))))
        .map(|reg_value| {
            unsafe {
                *value = reg_value; // SAFETY: The caller must guarantee that pointer is valid.
            }
            0
        })
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn set_register_value(
    ctx: *const libc::c_void,
    register: *const libc::c_char,
    value: u64,
) -> i64 {
    // SAFETY: The caller must guarantee that pointer is valid.
    let reg = match unsafe { CStr::from_ptr(register).to_str() } {
        Ok(v) => v,
        Err(_) => return -1,
    };

    let reg_selector = &mdbg_rs::RegSelector::Name(reg);
    Context::from(ctx as u64)
        .and_then(|mut ctx| {
            ctx.with_debugger(|d| d.set_register_value(reg_selector, value).or(Err(())))
        })
        .and(Ok(0))
        .unwrap_or(-1)
}

#[no_mangle]
pub extern "C" fn read_memory(ctx: *const libc::c_void, addr: u64, value: *mut i64) -> i64 {
    Context::from(ctx as u64)
        .and_then(|mut ctx| ctx.with_debugger(|d| d.read_memory(addr).or(Err(()))))
        .map(|mem_value| {
            unsafe {
                *value = mem_value; // SAFETY: The caller must guarantee that pointer is valid.
            }
            0
        })
        .unwrap_or(-1)
}
#[no_mangle]
pub extern "C" fn write_memory(ctx: *const libc::c_void, addr: u64, value: i64) -> i64 {
    Context::from(ctx as u64)
        .and_then(|mut ctx| ctx.with_debugger(|d| d.write_memory(addr, value).or(Err(()))))
        .and(Ok(0))
        .unwrap_or(-1)
}
