use rustyline::history::DefaultHistory;

use crate::helper;

pub fn run_command_loop(
    editor: &mut rustyline::Editor<helper::CliHelper, DefaultHistory>,
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
