
pub fn total_memory_mb() -> u32 {
    let mut system = sysinfo::System::new();
    system.refresh_memory();
    let mb = (system.total_memory() / (1024 * 1024)) as u32;
    mb.max(1024)
}
