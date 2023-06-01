use std::{
    borrow::BorrowMut,
    collections::HashMap,
    io::{self, BufRead},
    panic,
    process::exit,
};

use nix::{
    sys::{
        ptrace,
        wait::{self, waitpid},
    },
    unistd::Pid,
};

use crate::breakpoint::Breakpoint;

pub struct Debugger {
    program_name: String,
    program_pid: i32,
    breakpoints: HashMap<u64, Breakpoint>,
}

impl Debugger {
    pub fn new(program_name: String, program_pid: i32) -> Debugger {
        Debugger {
            program_name,
            program_pid,
            breakpoints: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        waitpid(Pid::from_raw(self.program_pid), None).expect("failed to wait pid");

        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line.expect("failed to read line");
            self.handle_command(line);
        }
    }

    fn handle_command(&mut self, line: String) {
        let args = line.split(" ").collect::<Vec<&str>>();
        let command = args[0];

        match command {
            "continue" => self.continue_execution(),
            "break" => {
                let addr = u64::from_str_radix(args[1], 16).expect("failed to parse address");
                self.set_breakpoint(addr);
            }
            _ => panic!("wrong command"),
        };
    }

    fn continue_execution(&self) {
        ptrace::cont(Pid::from_raw(self.program_pid), None).expect("failed to continue program");
        let status = waitpid(Pid::from_raw(self.program_pid), None).expect("failed to wait pid");
        if let wait::WaitStatus::Exited(_, status) = status {
            println!("Process exited with status: {}.", status);
            exit(0);
        }
    }

    fn set_breakpoint(&mut self, addr: u64) {
        let mut breakpoint = Breakpoint::new(self.program_pid, addr);
        breakpoint.switch(true);
        self.breakpoints.insert(addr, breakpoint);
    }
}
