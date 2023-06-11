mod breakpoint;
pub mod debugger;
mod reg;

use debugger::Debugger;
use nix::{
    libc::execl,
    sys::{personality, ptrace},
    unistd::{fork, ForkResult},
};
use std::{env::args, ffi::CString, ptr};

fn main() {
    let program_path = args().nth(1).expect("filepath isn't provided");

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            println!("Starting debugging process {}.", child);
            let mut debugger = Debugger::new(program_path, child.as_raw());
            debugger.run();
        }
        Ok(ForkResult::Child) => {
            ptrace::traceme().expect("failed to run traceme");
            let c_path = CString::new(program_path).unwrap();
            personality::set(personality::Persona::ADDR_NO_RANDOMIZE)
                .expect("failed to disable ASLR");
            unsafe {
                execl(c_path.as_ptr(), c_path.as_ptr(), ptr::null_mut::<i8>());
            }
        }
        Err(_) => println!("Fork failed"),
    }
}
