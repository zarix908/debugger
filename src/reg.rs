use std::collections::HashMap;

#[derive(PartialEq, Eq)]
#[allow(dead_code)]
pub enum Reg {
    R15,
    R14,
    R13,
    R12,
    RBP,
    RBX,
    R11,
    R10,
    R9,
    R8,
    RAX,
    RCX,
    RDX,
    RSI,
    RDI,
    RIP,
    CS,
    EFLAGS,
    RSP,
    SS,
    FSBASE,
    GSBASE,
    DS,
    ES,
    FS,
    GS,
}

struct RegDescriptor<'value, 'name> {
    reg: Reg,
    name: &'name str,
    value_ptr: &'value mut u64,
}

pub enum RegSelector<'a> {
    Reg(Reg),
    Name(&'a str),
}

pub fn set_register_value(
    regs: &mut nix::libc::user_regs_struct,
    selector: &RegSelector,
    value: u64,
) {
    select_regs(regs, Some(selector), Some(value));
}

pub fn get_register_value(regs: &mut nix::libc::user_regs_struct, selector: &RegSelector) -> u64 {
    *select_regs(regs, Some(selector), None)
        .unwrap()
        .values()
        .into_iter()
        .last()
        .unwrap()
}

pub fn dump_registers(regs: &mut nix::libc::user_regs_struct) -> HashMap<String, u64> {
    select_regs(regs, None, None).unwrap()
}

fn select_regs(
    regs: &mut nix::libc::user_regs_struct,
    selector: Option<&RegSelector>,
    set_value: Option<u64>,
) -> Option<HashMap<String, u64>> {
    let descpriptors = [
        RegDescriptor {
            reg: Reg::R15,
            name: "r15",
            value_ptr: &mut regs.r15,
        },
        RegDescriptor {
            reg: Reg::R14,
            name: "r14",
            value_ptr: &mut regs.r14,
        },
        RegDescriptor {
            reg: Reg::R13,
            name: "r13",
            value_ptr: &mut regs.r13,
        },
        RegDescriptor {
            reg: Reg::R12,
            name: "r12",
            value_ptr: &mut regs.r12,
        },
        RegDescriptor {
            reg: Reg::RBP,
            name: "rbp",
            value_ptr: &mut regs.rbp,
        },
        RegDescriptor {
            reg: Reg::RBX,
            name: "rbx",
            value_ptr: &mut regs.rbx,
        },
        RegDescriptor {
            reg: Reg::R11,
            name: "r11",
            value_ptr: &mut regs.r11,
        },
        RegDescriptor {
            reg: Reg::R10,
            name: "r10",
            value_ptr: &mut regs.r10,
        },
        RegDescriptor {
            reg: Reg::R9,
            name: "r9",
            value_ptr: &mut regs.r9,
        },
        RegDescriptor {
            reg: Reg::R8,
            name: "r8",
            value_ptr: &mut regs.r8,
        },
        RegDescriptor {
            reg: Reg::RAX,
            name: "rax",
            value_ptr: &mut regs.rax,
        },
        RegDescriptor {
            reg: Reg::RCX,
            name: "rcx",
            value_ptr: &mut regs.rcx,
        },
        RegDescriptor {
            reg: Reg::RDX,
            name: "rdx",
            value_ptr: &mut regs.rdx,
        },
        RegDescriptor {
            reg: Reg::RSI,
            name: "rsi",
            value_ptr: &mut regs.rsi,
        },
        RegDescriptor {
            reg: Reg::RDI,
            name: "rdi",
            value_ptr: &mut regs.rdi,
        },
        RegDescriptor {
            reg: Reg::RIP,
            name: "rip",
            value_ptr: &mut regs.rip,
        },
        RegDescriptor {
            reg: Reg::CS,
            name: "cs",
            value_ptr: &mut regs.cs,
        },
        RegDescriptor {
            reg: Reg::EFLAGS,
            name: "eflags",
            value_ptr: &mut regs.eflags,
        },
        RegDescriptor {
            reg: Reg::RSP,
            name: "rsp",
            value_ptr: &mut regs.rsp,
        },
        RegDescriptor {
            reg: Reg::SS,
            name: "ss",
            value_ptr: &mut regs.ss,
        },
        RegDescriptor {
            reg: Reg::FSBASE,
            name: "fsbase",
            value_ptr: &mut regs.fs_base,
        },
        RegDescriptor {
            reg: Reg::GSBASE,
            name: "gsbase",
            value_ptr: &mut regs.gs_base,
        },
        RegDescriptor {
            reg: Reg::DS,
            name: "ds",
            value_ptr: &mut regs.ds,
        },
        RegDescriptor {
            reg: Reg::ES,
            name: "es",
            value_ptr: &mut regs.es,
        },
        RegDescriptor {
            reg: Reg::FS,
            name: "fs",
            value_ptr: &mut regs.fs,
        },
        RegDescriptor {
            reg: Reg::GS,
            name: "gs",
            value_ptr: &mut regs.gs,
        },
    ];

    let mut result = HashMap::new();
    for decriptor in descpriptors {
        match &selector {
            Some(RegSelector::Reg(reg)) if decriptor.reg != *reg => continue,
            Some(RegSelector::Name(name)) if decriptor.name != *name => continue,
            _ => (),
        }

        match set_value {
            Some(value) => {
                *decriptor.value_ptr = value;
                break;
            }
            None => {
                result.insert(decriptor.name.to_owned(), *decriptor.value_ptr);
            }
        }
    }

    Some(result)
}
