use std::{collections::HashMap, os::raw::c_void};

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
    reg::{self, Reg, RegSelector},
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

    pub fn continue_execution(&mut self) -> Result<Option<i32>, String> {
        let pid = Pid::from_raw(self.program_pid);

        self.step_over_breakpoint()
            .map_err(|e| format!("failed to step over breakpoint: {}", e))?;

        ptrace::cont(pid, None).map_err(|e| format!("failed to continue program: {}", e))?;
        self.wait_trap(false)
            .map_err(|e| format!("failed to wait trap: {}", e))
    }

    pub fn set_breakpoint(&mut self, reference: BreakpointRef) -> Result<(), String> {
        let addr = match reference {
            BreakpointRef::Addr(addr) => Some(addr),
            BreakpointRef::Line { filename, line } => self
                .dwarf
                .get_source_line_addr(filename, line)
                .map_err(|e| format!("failed to get addr of source line: {}", e))?
                .map(|addr| addr + self.load_addr),
        };

        if let Some(addr) = addr {
            let breakpoint = self
                .breakpoints
                .entry(addr)
                .or_insert(Breakpoint::new(self.program_pid, addr));
            breakpoint
                .switch(true)
                .map_err(|e| format!("failed to enable breakpoint: {}", e))?;
        } else {
            Err("addr of source line not found")?;
        }

        Ok(())
    }

    pub fn get_register_value(&self, reg: &RegSelector) -> Result<u64, String> {
        let mut regs = ptrace::getregs(Pid::from_raw(self.program_pid))
            .map_err(|e| format!("failed to get regs: {}", e))?;

        Ok(reg::get_register_value(&mut regs, reg))
    }

    pub fn set_register_value(&self, reg: &RegSelector, value: u64) -> Result<(), String> {
        let mut regs = ptrace::getregs(Pid::from_raw(self.program_pid))
            .map_err(|e| format!("failed to get regs: {}", e))?;

        reg::set_register_value(&mut regs, reg, value);

        ptrace::setregs(Pid::from_raw(self.program_pid), regs)
            .map_err(|e| format!("failed to set regs: {}", e))?;
        Ok(())
    }

    pub fn dump_registers(&self) -> Result<HashMap<String, u64>, String> {
        let mut regs = ptrace::getregs(Pid::from_raw(self.program_pid))
            .map_err(|e| format!("failed to get regs: {}", e))?;

        Ok(reg::dump_registers(&mut regs))
    }

    pub fn read_memory(&self, addr: u64) -> Result<i64, String> {
        ptrace::read(Pid::from_raw(self.program_pid), addr as *mut c_void)
            .map_err(|e| format!("failed to read memory: {}", e))
    }

    pub fn write_memory(&self, addr: u64, value: i64) -> Result<(), String> {
        unsafe {
            ptrace::write(
                Pid::from_raw(self.program_pid),
                addr as *mut c_void,
                value as *mut c_void,
            )
            .map_err(|e| format!("failed to write memory: {}", e))
        }
    }

    fn step_over_breakpoint(&mut self) -> Result<(), String> {
        let rip = self
            .get_register_value(&RegSelector::Reg(Reg::RIP))
            .map_err(|e| format!("failed to get RIP register value: {}", e))?;

        match self.breakpoints.get_mut(&rip) {
            Some(bp) if bp.enabled() => {
                bp.switch(false)
                    .map_err(|e| format!("failed to disable breakpoint: {}", e))?;
            }
            _ => return Ok(()),
        }

        let pid = Pid::from_raw(self.program_pid);
        ptrace::step(pid, None).map_err(|e| format!("failed to single step program: {}", e))?;
        self.wait_trap(false)
            .map_err(|e| format!("failed to wait trap: {}", e))?;

        // redeclare bp due to reborrow self as mutable
        // unwrap because already check that breakpoint exists
        let bp = self.breakpoints.get_mut(&rip).unwrap();
        bp.switch(true)
            .map_err(|e| format!("failed to enable breakpoint: {}", e))?;

        Ok(())
    }

    pub fn wait_attach(&self) -> Result<(), String> {
        self.wait_trap(true).map(|_| ())
    }

    fn wait_trap(&self, si_code_must_user: bool) -> Result<Option<i32>, String> {
        let status = waitpid(Pid::from_raw(self.program_pid), None)
            .map_err(|e| format!("failed to wait pid: {}", e))?;

        match status {
            wait::WaitStatus::Stopped(_, Signal::SIGTRAP) => {
                const SI_USER: i32 = 0x0;
                const TRAP_BRKPT: i32 = 0x1;
                const TRAP_TRACE: i32 = 0x2;
                const SI_KERNEL: i32 = 0x80;

                let siginfo = ptrace::getsiginfo(Pid::from_raw(self.program_pid))
                    .map_err(|e| format!("failed to get siginfo: {}", e))?;

                if si_code_must_user && siginfo.si_code != SI_USER {
                    Err("could not attach to debugee process: wrong ci code")?;
                }

                match siginfo.si_code {
                    // hit breakpoint
                    SI_KERNEL | TRAP_BRKPT => {
                        let reg = RegSelector::Reg(Reg::RIP);
                        let rip = self
                            .get_register_value(&reg)
                            .map_err(|e| format!("failed to get RIP register value: {}", e))?;
                        self.set_register_value(&reg, rip - 1)
                            .map_err(|e| format!("failed to set value to RIP register: {}", e))?;
                    }

                    // traceme or signle step
                    SI_USER | TRAP_TRACE => (),

                    _ => Err(format!("Uknown SIGTRAP code: {}", siginfo.si_code))?,
                }
            }
            wait::WaitStatus::Signaled(_, Signal::SIGSEGV, _) => {
                Err("Segfault occured.")?;
            }
            wait::WaitStatus::Exited(_, status) => {
                return Ok(Some(status));
            }
            _ => Err(format!("Uknown signal: {:?}", status))?,
        }

        Ok(None)
    }
}

pub enum BreakpointRef {
    Addr(u64),
    Line { filename: String, line: u64 },
}
