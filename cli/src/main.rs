mod args;
mod commands;
mod helper;

use std::{env::var, ffi::CString, fs::create_dir_all, path::Path, ptr};

use args::{Args, Commands};
use clap::Parser;
use nix::{
    libc::execl,
    sys::ptrace,
    unistd::{fork, ForkResult, Pid},
};
use rustyline::history::DefaultHistory;

use crate::commands::run_command_loop;

fn main() {
    parse_args().unwrap();
}

fn parse_args() -> Result<(), String> {
    let args = Args::parse();
    match args.commands {
        Commands::Run { program_path } => run(program_path),
        Commands::Attach { program_path, pid } => attach(program_path, pid),
    }
}

fn run(program_path: String) -> Result<(), String> {
    // SAFETY: Call in single thread.
    match unsafe { fork() } {
        Ok(ForkResult::Parent { child }) => init_debugger(program_path, child.as_raw())?,
        Ok(ForkResult::Child) => {
            ptrace::traceme().expect("failed to run traceme");
            let c_path = CString::new(program_path.clone()).unwrap();
            let c_path_arg = CString::new(program_path).unwrap();

            // SAFETY: there isn't aliased pointers.
            unsafe {
                execl(c_path.as_ptr(), c_path_arg.as_ptr(), ptr::null::<i8>());
            }
        }
        Err(e) => Err(format!("failed to fork process: {}", e))?,
    }

    Ok(())
}

fn attach(program_path: String, pid: i32) -> Result<(), String> {
    ptrace::attach(Pid::from_raw(pid)).map_err(|e| format!("failed to attach to process {}", e))?;
    init_debugger(program_path, pid)
}

fn init_debugger(program_path: String, pid: i32) -> Result<(), String> {
    let mut debugger = mdbg_rs::load_in_memory(pid, &program_path)?;

    debugger
        .wait_attach()
        .map_err(|e| format!("failed to wait trap: {}", e))?;
    println!("Starting debugging process {}.", pid);
    let load_addr = mdbg_rs::linux_maps::get_load_addr(pid, &program_path)
        .map_err(|e| format!("failed to get load addr: {}", e))?;
    debugger.set_load_addr(load_addr);

    let mut editor = rustyline::Editor::<helper::CliHelper, DefaultHistory>::new()
        .map_err(|e| format!("failed to create editor: {}", e))?;

    editor.set_helper(Some(helper::CliHelper::new(
        vec![
            "continue",
            "break",
            "register dump",
            "register read",
            "register write",
            "memory read",
            "memory write",
        ]
        .into_iter()
        .map(|c| c.to_owned())
        .collect(),
    )));

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
        create_dir_all(&parent)
            .map_err(|e| format!("failed to create directory to save command history: {}", e))?;
        editor
            .save_history(&history_path)
            .map_err(|e| format!("failed to save history: {}", e))?;
    } else {
        run_command_loop(&mut editor, &mut debugger)?;
    }

    Ok(())
}
