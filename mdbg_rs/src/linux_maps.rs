use regex::Regex;
use std::fs::read_to_string;

pub fn get_load_addr(pid: i32, executable_path: &str) -> Result<u64, String> {
    let maps = read_to_string(format!("/proc/{}/maps", pid))
        .map_err(|e| format!("failed to read maps file {}: {}", executable_path, e))?;

    let regexp = Regex::new(&format!("([0-9a-f]*)-.*{}", executable_path))
        .expect("failed to compile regexp");
    Ok(regexp
        .captures_iter(&maps)
        .map(|c| c.get(1).unwrap().as_str())
        .map(|addr| u64::from_str_radix(addr, 16).expect("failed to parse address"))
        .min()
        .unwrap())
}
