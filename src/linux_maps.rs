use regex::Regex;
use std::fs::read_to_string;

pub fn get_load_addr(pid: i32, executable_path: &str) -> u64 {
    let regexp = Regex::new(&format!("(\\d*)-.*{}", executable_path)).unwrap();
    let maps = read_to_string(format!("/proc/{}/maps", pid)).expect("failed to read maps file");

    regexp
        .captures_iter(&maps)
        .map(|c| c.get(1).unwrap().as_str())
        .map(|addr| u64::from_str_radix(addr, 16).expect("failed to parse address"))
        .min()
        .unwrap()
}
