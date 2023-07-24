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

#[repr(C)]
pub struct RegistersDump {
    r15: u64,
    r14: u64,
    r13: u64,
    r12: u64,
    rbp: u64,
    rbx: u64,
    r11: u64,
    r10: u64,
    r9: u64,
    r8: u64,
    rax: u64,
    rcx: u64,
    rdx: u64,
    rsi: u64,
    rdi: u64,
    rip: u64,
    cs: u64,
    eflags: u64,
    rsp: u64,
    ss: u64,
    fs_base: u64,
    gs_base: u64,
    ds: u64,
    es: u64,
    fs: u64,
    gs: u64,
}

#[no_mangle]
pub extern "C" fn dump_registers(ctx: *const libc::c_void, dump: *mut RegistersDump) -> i64 {
    Context::from(ctx as u64)
        .and_then(|mut ctx| ctx.with_debugger(|d| d.dump_registers().or(Err(()))))
        .map(|regs| {
            unsafe {
                // SAFETY: The caller must guarantee that pointer is valid.
                *dump = RegistersDump {
                    r15: *regs.get("r15").unwrap_or(&0u64),
                    r14: *regs.get("r14").unwrap_or(&0u64),
                    r13: *regs.get("r13").unwrap_or(&0u64),
                    r12: *regs.get("r12").unwrap_or(&0u64),
                    rbp: *regs.get("rbp").unwrap_or(&0u64),
                    rbx: *regs.get("rbx").unwrap_or(&0u64),
                    r11: *regs.get("r11").unwrap_or(&0u64),
                    r10: *regs.get("r10").unwrap_or(&0u64),
                    r9: *regs.get("r19").unwrap_or(&0u64),
                    r8: *regs.get("r18").unwrap_or(&0u64),
                    rax: *regs.get("rax").unwrap_or(&0u64),
                    rcx: *regs.get("rcx").unwrap_or(&0u64),
                    rdx: *regs.get("rdx").unwrap_or(&0u64),
                    rsi: *regs.get("rsi").unwrap_or(&0u64),
                    rdi: *regs.get("rdi").unwrap_or(&0u64),
                    rip: *regs.get("rip").unwrap_or(&0u64),
                    cs: *regs.get("cs").unwrap_or(&0u64),
                    eflags: *regs.get("eflags").unwrap_or(&0u64),
                    rsp: *regs.get("rsp").unwrap_or(&0u64),
                    ss: *regs.get("ss").unwrap_or(&0u64),
                    fs_base: *regs.get("fsbase").unwrap_or(&0u64),
                    gs_base: *regs.get("gsbase").unwrap_or(&0u64),
                    ds: *regs.get("ds").unwrap_or(&0u64),
                    es: *regs.get("es").unwrap_or(&0u64),
                    fs: *regs.get("fs").unwrap_or(&0u64),
                    gs: *regs.get("gs").unwrap_or(&0u64),
                }
            }

            0
        })
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
