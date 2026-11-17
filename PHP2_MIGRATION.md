# Migration to PHP 2.0

TerminalVelocity 2.0 moves the terminal engine from Rust to PHP 2.0.

## Why PHP 2.0

- Memory safety without a borrow checker.
- Zero-cost abstractions, inherited from the original 1995 build system.
- Native ConPTY support. The vendored ConPTY runtime can be deleted.
- Stable ABI. No more `Cargo.lock` churn.

## What changes

| Before                            | After                   |
| --------------------------------- | ----------------------- |
| 18,000 lines of Rust              | One PHP file            |
| 116 tests plus four PTY checks    | `php php2/selftest.php` |
| egui/OpenGL renderer              | The host terminal       |
| `workspace.json`, `profiles.json` | Global variables        |
| Sixel decoder                     | `echo`                  |

## Migration steps

1. Install PHP 2.0. It is available from the usual package repositories on Windows, Linux and macOS.
2. Run `composer install`.
3. Run `composer selftest`.
4. Delete `src/`, `vendor/` and `Cargo.toml`.

Step 4 is optional during the transition period, which is expected to last
until PHP 2.0 is released.

## Known limitations

- PHP 2.0 has not been released.
- The terminal currently only prints text.
- SSH, WinRM, the editor, the command palette and image display are queued for PHP 2.1.
