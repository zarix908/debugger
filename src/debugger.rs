use std::{
    collections::HashMap,
    io::{self, BufRead},
    os::raw::c_void,
    panic,
    process::exit,
};

use nix::{
    sys::{
        ptrace,
        signal::Signal,
        wait::{self, waitpid},
    },
    unistd::Pid,
};

use crate::{
    breakpoint::Breakpoint,
    dwarf::Dwarf,
    reg::{Reg, RegSelector},
};

pub struct Debugger<'a> {
    program_pid: i32,
    dwarf: Dwarf<'a>,
    load_addr: u64,
    breakpoints: HashMap<u64, Breakpoint>,
}

impl<'a> Debugger<'a> {
    pub fn new(program_pid: i32, dwarf: Dwarf<'a>, load_addr: u64) -> Debugger<'a> {
        Debugger {
            program_pid,
            dwarf,
            load_addr,
            breakpoints: HashMap::new(),
        }
    }

    pub fn run(&mut self) {
        println!("Starting debugging process {}.", self.program_pid);

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
                let line = args[2]
                    .parse::<u64>()
                    .expect("failed to parse source line number");
                self.set_breakpoint(BreakpointRef::Line {
                    filename: args[1].to_owned(),
                    line,
                });
            }
            "register" => {
                match args[1] {
                    "dump" => self.dump_registers(),
                    "read" => println!(
                        "{}: {:#X}",
                        args[2],
                        self.get_register_value(&RegSelector::Name(args[2]))
                    ),
                    "write" => {
                        let value =
                            u64::from_str_radix(args[3], 16).expect("failed to parse value");
                        self.set_register_value(&RegSelector::Name(args[2]), value)
                    }
                    _ => panic!("wrong command"),
                };
            }
            "memory" => {
                let addr = u64::from_str_radix(args[2], 16).expect("failed to parse address");

                match args[1] {
                    "read" => println!("{:#X}", self.read_memory(addr)),
                    "write" => {
                        let value =
                            i64::from_str_radix(args[3], 16).expect("failed to parse value");
                        self.write_memory(addr, value);
                    }
                    _ => panic!("wrong command"),
                }
            }
            _ => panic!("wrong command"),
        };
    }

    fn continue_execution(&mut self) {
        let pid = Pid::from_raw(self.program_pid);

        self.step_over_breakpoint();

        ptrace::cont(pid, None).expect("failed to continue program");
        self.wait_trap()
    }

    fn set_breakpoint(&mut self, reference: BreakpointRef) {
        let addr = match reference {
            BreakpointRef::Addr(addr) => Some(addr),
            BreakpointRef::Line { filename, line } => self
                .dwarf
                .get_source_line_addr(filename, line)
                .map(|addr| addr + self.load_addr),
        };

        if let Some(addr) = addr {
            let breakpoint = self
                .breakpoints
                .entry(addr)
                .or_insert(Breakpoint::new(self.program_pid, addr));
            breakpoint.switch(true);
        } else {
            println!("addr of source line not found");
        }
    }

    fn get_register_value(&self, reg: &RegSelector) -> u64 {
        let regs = ptrace::getregs(Pid::from_raw(self.program_pid)).expect("failed to get regs");

        match reg {
            RegSelector::Reg(Reg::R15) | RegSelector::Name("r15") => regs.r15,
            RegSelector::Reg(Reg::R14) | RegSelector::Name("r14") => regs.r14,
            RegSelector::Reg(Reg::R13) | RegSelector::Name("r13") => regs.r13,
            RegSelector::Reg(Reg::R12) | RegSelector::Name("r12") => regs.r12,
            RegSelector::Reg(Reg::RBP) | RegSelector::Name("rbp") => regs.rbp,
            RegSelector::Reg(Reg::RBX) | RegSelector::Name("rbx") => regs.rbx,
            RegSelector::Reg(Reg::R11) | RegSelector::Name("r11") => regs.r11,
            RegSelector::Reg(Reg::R10) | RegSelector::Name("r10") => regs.r10,
            RegSelector::Reg(Reg::R9) | RegSelector::Name("r9") => regs.r9,
            RegSelector::Reg(Reg::R8) | RegSelector::Name("r8") => regs.r8,
            RegSelector::Reg(Reg::RAX) | RegSelector::Name("rax") => regs.rax,
            RegSelector::Reg(Reg::RCX) | RegSelector::Name("rcx") => regs.rcx,
            RegSelector::Reg(Reg::RDX) | RegSelector::Name("rdx") => regs.rdx,
            RegSelector::Reg(Reg::RSI) | RegSelector::Name("rsi") => regs.rsi,
            RegSelector::Reg(Reg::RDI) | RegSelector::Name("rdi") => regs.rdi,
            RegSelector::Reg(Reg::RIP) | RegSelector::Name("rip") => regs.rip,
            RegSelector::Reg(Reg::CS) | RegSelector::Name("cs") => regs.cs,
            RegSelector::Reg(Reg::EFLAGS) | RegSelector::Name("eflags") => regs.eflags,
            RegSelector::Reg(Reg::RSP) | RegSelector::Name("rsp") => regs.rsp,
            RegSelector::Reg(Reg::SS) | RegSelector::Name("ss") => regs.ss,
            RegSelector::Reg(Reg::FSBASE) | RegSelector::Name("fsbase") => regs.fs_base,
            RegSelector::Reg(Reg::GSBASE) | RegSelector::Name("gsbase") => regs.gs_base,
            RegSelector::Reg(Reg::DS) | RegSelector::Name("ds") => regs.ds,
            RegSelector::Reg(Reg::ES) | RegSelector::Name("es") => regs.es,
            RegSelector::Reg(Reg::FS) | RegSelector::Name("fs") => regs.fs,
            RegSelector::Reg(Reg::GS) | RegSelector::Name("gs") => regs.gs,
            _ => panic!("register not found"),
        }
    }

    fn set_register_value(&self, reg: &RegSelector, value: u64) {
        let mut regs =
            ptrace::getregs(Pid::from_raw(self.program_pid)).expect("failed to get regs");

        match reg {
            RegSelector::Reg(Reg::R15) | RegSelector::Name("r15") => regs.r15 = value,
            RegSelector::Reg(Reg::R14) | RegSelector::Name("r14") => regs.r14 = value,
            RegSelector::Reg(Reg::R13) | RegSelector::Name("r13") => regs.r13 = value,
            RegSelector::Reg(Reg::R12) | RegSelector::Name("r12") => regs.r12 = value,
            RegSelector::Reg(Reg::RBP) | RegSelector::Name("rbp") => regs.rbp = value,
            RegSelector::Reg(Reg::RBX) | RegSelector::Name("rbx") => regs.rbx = value,
            RegSelector::Reg(Reg::R11) | RegSelector::Name("r11") => regs.r11 = value,
            RegSelector::Reg(Reg::R10) | RegSelector::Name("r10") => regs.r10 = value,
            RegSelector::Reg(Reg::R9) | RegSelector::Name("r9") => regs.r9 = value,
            RegSelector::Reg(Reg::R8) | RegSelector::Name("r8") => regs.r8 = value,
            RegSelector::Reg(Reg::RAX) | RegSelector::Name("rax") => regs.rax = value,
            RegSelector::Reg(Reg::RCX) | RegSelector::Name("rcx") => regs.rcx = value,
            RegSelector::Reg(Reg::RDX) | RegSelector::Name("rdx") => regs.rdx = value,
            RegSelector::Reg(Reg::RSI) | RegSelector::Name("rsi") => regs.rsi = value,
            RegSelector::Reg(Reg::RDI) | RegSelector::Name("rdi") => regs.rdi = value,
            RegSelector::Reg(Reg::RIP) | RegSelector::Name("rip") => regs.rip = value,
            RegSelector::Reg(Reg::CS) | RegSelector::Name("cs") => regs.cs = value,
            RegSelector::Reg(Reg::EFLAGS) | RegSelector::Name("eflags") => regs.eflags = value,
            RegSelector::Reg(Reg::RSP) | RegSelector::Name("rsp") => regs.rsp = value,
            RegSelector::Reg(Reg::SS) | RegSelector::Name("ss") => regs.ss = value,
            RegSelector::Reg(Reg::FSBASE) | RegSelector::Name("fsbase") => regs.fs_base = value,
            RegSelector::Reg(Reg::GSBASE) | RegSelector::Name("gsbase") => regs.gs_base = value,
            RegSelector::Reg(Reg::DS) | RegSelector::Name("ds") => regs.ds = value,
            RegSelector::Reg(Reg::ES) | RegSelector::Name("es") => regs.es = value,
            RegSelector::Reg(Reg::FS) | RegSelector::Name("fs") => regs.fs = value,
            RegSelector::Reg(Reg::GS) | RegSelector::Name("gs") => regs.gs = value,
            _ => panic!("register not found"),
        };

        ptrace::setregs(Pid::from_raw(self.program_pid), regs).expect("failed to set regs");
    }

    fn dump_registers(&self) {
        let regs = ptrace::getregs(Pid::from_raw(self.program_pid)).expect("failed to get regs");

        let regs = [
            regs.r15,
            regs.r14,
            regs.r13,
            regs.r12,
            regs.rbp,
            regs.rbx,
            regs.r11,
            regs.r10,
            regs.r9,
            regs.r8,
            regs.rax,
            regs.rcx,
            regs.rdx,
            regs.rsi,
            regs.rdi,
            regs.rip,
            regs.cs,
            regs.eflags,
            regs.rsp,
            regs.ss,
            regs.fs_base,
            regs.gs_base,
            regs.ds,
            regs.es,
            regs.fs,
            regs.gs,
        ];

        let names = [
            "r15", "r14", "r13", "r12", "rbp", "rbx", "r11", "r10", "r9", "r8", "rax", "rcx",
            "rdx", "rsi", "rdi", "rip", "cs", "eflags", "rsp", "ss", "fsbase", "gsbase", "ds",
            "es", "fs", "gs",
        ];

        for i in 0..26 {
            println!("{}: {:#X}", names[i], regs[i]);
        }
    }

    fn read_memory(&self, addr: u64) -> i64 {
        ptrace::read(Pid::from_raw(self.program_pid), addr as *mut c_void)
            .expect("failed to peek instruction")
    }

    fn write_memory(&self, addr: u64, value: i64) {
        unsafe {
            ptrace::write(
                Pid::from_raw(self.program_pid),
                addr as *mut c_void,
                value as *mut c_void,
            )
            .expect("failed to peek instruction");
        };
    }

    fn step_over_breakpoint(&mut self) {
        let rip = self.get_register_value(&RegSelector::Reg(Reg::RIP));

        match self.breakpoints.get_mut(&rip) {
            Some(bp) if bp.enabled() => {
                bp.switch(false);
            }
            _ => return,
        }

        let pid = Pid::from_raw(self.program_pid);
        ptrace::step(pid, None).expect("failed to single step program");
        self.wait_trap();

        let bp = self.breakpoints.get_mut(&rip).unwrap();
        bp.switch(true);
    }

    fn wait_trap(&self) {
        let status = waitpid(Pid::from_raw(self.program_pid), None).expect("failed to wait pid");
        match status {
            wait::WaitStatus::Stopped(_, Signal::SIGTRAP) => {
                const SI_KERNEL: i32 = 0x80;
                const TRAP_BRKPT: i32 = 0x1;
                const TRAP_TRACE: i32 = 0x2;

                let siginfo = ptrace::getsiginfo(Pid::from_raw(self.program_pid))
                    .expect("failed to get siginfo");

                match siginfo.si_code {
                    // hit breakpoint
                    SI_KERNEL | TRAP_BRKPT => {
                        let reg = RegSelector::Reg(Reg::RIP);
                        let rip = self.get_register_value(&reg);
                        self.set_register_value(&reg, rip - 1);
                    }

                    // signle step
                    TRAP_TRACE => (),

                    _ => println!("Uknown SIGTRAP code: {}", siginfo.si_code),
                }
            }
            wait::WaitStatus::Signaled(_, Signal::SIGSEGV, _) => {
                println!("Segfault occured.");
                exit(255);
            }
            wait::WaitStatus::Exited(_, status) => {
                println!("Process exited with status: {}.", status);
                exit(0);
            }
            _ => println!("Got signal: {:?}", status),
        }
    }
}

enum BreakpointRef {
    Addr(u64),
    Line { filename: String, line: u64 },
}
