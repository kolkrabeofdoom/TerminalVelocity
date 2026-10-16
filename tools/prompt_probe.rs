// Offline console helper for exact Windows argument and working-directory tests.
fn main() {
    for arg in std::env::args().skip(1) {
        let hex: String = arg
            .as_bytes()
            .iter()
            .map(|byte| format!("{byte:02X}"))
            .collect();
        println!("AI_ARG:{hex}");
    }
    println!("AI_CWD:{}", std::env::current_dir().unwrap().display());
    // Let ConPTY flush its output before signalling process exit to the parent.
    std::thread::sleep(std::time::Duration::from_millis(500));
}
