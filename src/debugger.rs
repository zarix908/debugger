use std::{
    io::{self, BufRead},
    panic,
};

use nix::{
    sys::{ptrace, wait::waitpid},
    unistd::Pid,
};

pub struct Debugger {
    program_name: String,
    program_pid: Pid,
}

impl Debugger {
    pub fn new(program_name: String, program_pid: Pid) -> Debugger {
        Debugger {
            program_name,
            program_pid,
        }
    }

    pub fn run(&self) {
        waitpid(self.program_pid, None).expect("failed to wait pid");

        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            let line = line.expect("failed to read line");
            self.handle_command(line);
        }
    }

    fn handle_command(&self, line: String) {
        let args = line.split(" ").collect::<Vec<&str>>();
        let command = args[0];

        match command {
            "continue" => self.continue_execution(),
            _ => panic!("wrong command"),
        };
    }

    fn continue_execution(&self) {
        ptrace::cont(self.program_pid, None).expect("failed to continue program");
    }
}
