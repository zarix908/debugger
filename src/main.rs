mod breakpoint;
pub mod debugger;
mod dwarf;
mod linux_maps;
mod reg;

use debugger::Debugger;
use nix::{
    libc::execl,
    sys::{personality, ptrace},
    unistd::{fork, ForkResult},
};
use std::{borrow::Borrow, env::args, ffi::CString, fs, ops::Deref, ptr};

use crate::dwarf::{borrow_section, load_dwarf};

fn main() {
    let program_path = args().nth(1).expect("filepath isn't provided");

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let file = fs::File::open(&program_path).unwrap();
            let mmap = unsafe { memmap::Mmap::map(&file).unwrap() };
            let (dwarf, endian) = load_dwarf(mmap.deref().borrow());
            let dwarf = borrow_section(&dwarf, endian);

            let load_addr = linux_maps::get_load_addr(child.as_raw(), &program_path);

            let mut debugger = Debugger::new(child.as_raw(), dwarf, load_addr);
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
