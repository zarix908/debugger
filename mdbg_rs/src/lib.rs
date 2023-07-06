mod breakpoint;
mod debugger;
mod dwarf;
mod linux_maps;
mod reg;

use std::fs;

pub use debugger::{BreakpointRef, Debugger};
use dwarf::Dwarf;
pub use reg::{Reg, RegSelector};

pub fn load_in_memory(program_pid: i32, program_path: &str) -> Result<Debugger<'static>, String> {
    let file = fs::File::open(program_path)
        .map_err(|e| format!("failed to open file {}: {}", program_path, e))?;
    let mmap =
        unsafe { memmap::Mmap::map(&file).map_err(|e| format!("failed to mmap file: {}", e))? };

    let (dwarf, endian) = dwarf::load_dwarf(Box::leak(Box::new(mmap)))?;
    let dwarf = Dwarf::new(dwarf::borrow_section(Box::leak(Box::new(dwarf)), endian));

    let load_addr = linux_maps::get_load_addr(program_pid, &program_path)
        .map_err(|e| format!("failed to get load addr: {}", e))?;

    Ok(Debugger::new(program_pid, dwarf, load_addr))
}
