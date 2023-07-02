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

pub enum RegSelector<'a> {
    Reg(Reg),
    Name(&'a str),
}
