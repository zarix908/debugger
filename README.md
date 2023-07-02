# mDBG-RS

## Description

mDBG-RS is a simple debugger for learning purposes, written according to the [@TartanLlama](https://github.com/TartanLlama) guide.

## Working logic

There are three basic modules.   
  
The breakpoint module enables and disables a breakpoint at a specified address. The switch method pulls an assemble instruction at a specified address (using nix::ptrace) and modifies it's opcode to int3. An original instruction is stored at the breakpoint. The modified instruction interrupts a debuggee program and the OS gives control back to the debugger. To disable the breakpoint the switch method restores the previously stored original instrution.  

The debugger module accepts a set of commands and executes them using nix::ptrace.  
  
The dwarf module parses the DWARF formatted info from an ELF file and provides a source mapping to the debugger. A source line address is calculated as the sum of the executable load address and the DWARF stored line address. The module uses gimli crate to parse DWARF.  
