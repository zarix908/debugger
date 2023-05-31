pub mod debugger;

use debugger::Debugger;
use nix::{
    libc::execl,
    sys::ptrace,
    unistd::{fork, ForkResult},
};
use std::{env::args, ffi::CString, ptr};

fn main() {
    let program_path = args().nth(1).expect("filepath isn't provided");

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            println!("Starting debugging process {}", child);
            let debugger = Debugger::new(program_path, child);
            debugger.run();
        }
        Ok(ForkResult::Child) => {
            ptrace::traceme().expect("failed to run traceme");
            let c_path = CString::new(program_path).unwrap();
            unsafe {
                execl(c_path.as_ptr(), c_path.as_ptr(), ptr::null_mut::<i8>());
            }
        }
        Err(_) => println!("Fork failed"),
    }
}
