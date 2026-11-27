//! PTY test modes for CI and manual diagnosis. Compiled only with the `test-tools`
//! feature, so release packages do not contain them.

mod directory;
mod handoff;
mod profile;
mod restart;
mod search;
mod smoke;

type TestMode = fn() -> Result<(), Box<dyn std::error::Error>>;

/// Argument, test function and report file name, checked in this order.
const MODES: [(&str, TestMode, &str); 6] = [
    ("--search-test", search::search_test, "search-test.txt"),
    ("--profile-test", profile::profile_test, "profile-test.txt"),
    ("--handoff-test", handoff::handoff_test, "handoff-test.txt"),
    ("--restart-test", restart::restart_test, "restart-test.txt"),
    (
        "--directory-test",
        directory::directory_test,
        "directory-test.txt",
    ),
    ("--smoke-test", smoke::smoke_test, "shell-test.txt"),
];

/// Run the first requested test mode. On failure the error is written to the mode's
/// report file next to the executable, as before. Returns the exit code, or `None`
/// when no test mode was requested.
pub fn run_if_requested() -> Option<i32> {
    let (_, test, report) = MODES
        .iter()
        .find(|(argument, _, _)| std::env::args().any(|arg| arg == *argument))?;
    match test() {
        Ok(()) => Some(0),
        Err(error) => {
            let path = std::env::current_exe()
                .unwrap_or_default()
                .with_file_name(report);
            let _ = std::fs::write(path, format!("FEHLER: {error}"));
            Some(1)
        }
    }
}
