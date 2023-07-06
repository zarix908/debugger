use std::{
    env::{args, var},
    ffi::CString,
    fs::create_dir_all,
    path::Path,
    ptr,
};

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

            let mut editor = rustyline::DefaultEditor::new()
                .map_err(|e| format!("failed to create editor: {}", e))?;

            let history_path = var("HOME").ok().map(|home| {
                Path::new(&home)
                    .join(".cache")
                    .join("mdbg_rs")
                    .join("history")
            });
            if let Some(history_path) = history_path {
                let _ = editor.load_history(&history_path);
                run_command_loop(&mut editor, &mut debugger)?;
                let parent = history_path.parent().unwrap();
                create_dir_all(&parent).map_err(|e| {
                    format!("failed to create directory to save command history: {}", e)
                })?;
                editor
                    .save_history(&history_path)
                    .map_err(|e| format!("failed to save history: {}", e))?;
            } else {
                run_command_loop(&mut editor, &mut debugger)?;
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
        Err(e) => Err(format!("failed to fork process: {}", e))?,
    }

    Ok(())
}

fn run_command_loop(
    editor: &mut rustyline::DefaultEditor,
    debugger: &mut mdbg_rs::Debugger,
) -> Result<(), String> {
    loop {
        let readline = editor.readline("mdbg> ");
        match readline {
            Ok(line) => {
                editor
                    .add_history_entry(line.as_str())
                    .map_err(|e| format!("failed to add history entry: {}", e))?;

                if let Some(status) = handle_command(debugger, line)? {
                    println!("Process exited with status: {}", status);
                    break;
                }
            }
            Err(rustyline::error::ReadlineError::Interrupted) => {
                println!("CTRL-C");
                break;
            }
            Err(rustyline::error::ReadlineError::Eof) => {
                println!("CTRL-D");
                break;
            }
            Err(e) => Err(format!("failed to read line: {}", e))?,
        }
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
                        .get_register_value(&mdbg_rs::RegSelector::Name(args[2]))
                        .map_err(|e| format!("failed to get register value: {}", e))?
                ),
                "write" => {
                    let value = u64::from_str_radix(args[3], 16)
                        .map_err(|e| format!("failed to parse hex value: {}", e))?;
                    debugger
                        .set_register_value(&mdbg_rs::RegSelector::Name(args[2]), value)
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
