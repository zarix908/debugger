use std::{
    env::args,
    ffi::CString,
    io::{self, BufRead},
    ptr,
};

use mdbg_rs::RegSelector;
use nix::{
    libc::execl,
    sys::{personality, ptrace},
    unistd::{fork, ForkResult},
};

fn main() {
    run().unwrap();
}

fn run() -> Result<(), String> {
    let program_path = args().nth(1).ok_or_else(|| "filepath isn't provided")?;

    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => {
            let mut debugger = mdbg_rs::load_in_memory(child.as_raw(), &program_path)?;

            println!("Starting debugging process {}.", child.as_raw());

            debugger
                .wait_attach()
                .map_err(|e| format!("failed to wait trap: {}", e))?;

            let stdin = io::stdin();
            for line in stdin.lock().lines() {
                let line = line.map_err(|e| format!("failed to read line: {}", e))?;
                if let Some(status) = handle_command(&mut debugger, line)
                    .map_err(|e| format!("failed to handle command: {}", e))?
                {
                    println!("Process exited with status: {}", status);
                    break;
                };
            }

            return Ok(());
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

    Ok(())
}

fn handle_command(debugger: &mut mdbg_rs::Debugger, line: String) -> Result<Option<i32>, String> {
    let args = line.split(" ").collect::<Vec<&str>>();
    let command = args[0];

    let mut exit = None;
    match command {
        "continue" => {
            exit = debugger
                .continue_execution()
                .map_err(|e| format!("failed to continue execution: {}", e))?
        }
        "break" => {
            let line = args[2]
                .parse::<u64>()
                .map_err(|e| format!("failed to parse source line number: {}", e))?;
            debugger
                .set_breakpoint(mdbg_rs::BreakpointRef::Line {
                    filename: args[1].to_owned(),
                    line,
                })
                .map_err(|e| format!("failed to set breakpoint: {}", e))?;
        }
        "register" => {
            match args[1] {
                "dump" => {
                    for (reg, val) in debugger
                        .dump_registers()
                        .map_err(|e| format!("failed to dump registers: {}", e))?
                    {
                        println!("{}: {:#X}", reg, val)
                    }
                }
                "read" => println!(
                    "{}: {:#X}",
                    args[2],
                    debugger
                        .get_register_value(&RegSelector::Name(args[2]))
                        .map_err(|e| format!("failed to get register value: {}", e))?
                ),
                "write" => {
                    let value = u64::from_str_radix(args[3], 16)
                        .map_err(|e| format!("failed to parse hex value: {}", e))?;
                    debugger
                        .set_register_value(&RegSelector::Name(args[2]), value)
                        .map_err(|e| format!("failed to set value to register: {}", e))?
                }
                _ => panic!("wrong command"),
            };
        }
        "memory" => {
            let addr = u64::from_str_radix(args[2], 16)
                .map_err(|e| format!("failed to parse memory address: {}", e))?;

            match args[1] {
                "read" => println!("{:#X}", debugger.read_memory(addr)?),
                "write" => {
                    let value = i64::from_str_radix(args[3], 16)
                        .map_err(|e| format!("failed to parse hex value: {}", e))?;
                    debugger.write_memory(addr, value)?;
                }
                _ => panic!("wrong command"),
            }
        }
        _ => panic!("wrong command"),
    };

    Ok(exit)
}
