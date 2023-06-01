use std::os::raw::c_void;

use nix::{sys::ptrace, unistd::Pid};

pub struct Breakpoint {
    program_pid: i32,
    addr: u64,
    enabled: bool,
    replaced_instruction_opcode: Option<u8>,
}

impl Breakpoint {
    pub fn new(program_pid: i32, addr: u64) -> Breakpoint {
        Breakpoint {
            program_pid,
            addr,
            enabled: false,
            replaced_instruction_opcode: None,
        }
    }

    pub fn switch(&mut self, enable: bool) {
        let pid = Pid::from_raw(self.program_pid);
        let instruction =
            ptrace::read(pid, self.addr as *mut c_void).expect("failed to peek instruction");

        let replaced_instruction = if enable {
            self.replaced_instruction_opcode = Some((instruction & 0xFF) as u8);
            const INT3_OPCODE: i64 = 0xCC;
            (instruction & !0xFF) | INT3_OPCODE
        } else {
            (instruction & !0xFF)
                | self.replaced_instruction_opcode.expect(
                    "failed to disable breakpoint: opcode of replaced instruction isn't saved",
                ) as i64
        };

        unsafe {
            ptrace::write(
                pid,
                self.addr as *mut c_void,
                replaced_instruction as *mut c_void,
            )
            .expect("failed to poke breakpoint instruction");
        };

        self.enabled = enable;
    }
}
