use colored::Colorize;
use crossterm::terminal::{disable_raw_mode, Clear, ClearType};
use crossterm::{cursor, execute};
use std::fmt::Display;
use std::{io, process};

#[cfg(windows)]
use crate::debug_println;
#[cfg(windows)]
use std::ffi::OsStr;
#[cfg(windows)]
use std::io::StdoutLock;
use anyhow::Error;


pub fn init() {
    enable_color_on_windows();
    clear_screen();
}
fn enable_color_on_windows() {
    #[cfg(windows)]
    colored::control::set_virtual_terminal(true).unwrap();
}

pub(crate) fn clear_screen() {
    execute!(
        io::stdout(),
        Clear(ClearType::All),
        cursor::MoveTo(0, 0),
        cursor::Hide
    )
    .unwrap();
}

pub(crate) fn quit(e: Error) -> ! {
    disable_raw_mode().unwrap();
    execute!(io::stdout(), cursor::Show).unwrap();
    #[cfg(windows)]
    asio_kill();

    println!("{e}");
    process::exit(0);
}

#[cfg(windows)]
fn try_get_current_executable_name() -> Option<String> {
    std::env::current_exe()
        .ok()?
        .file_name()?
        .to_str()?
        .to_owned()
        .into()
}

#[cfg(windows)]
pub fn asio_kill() {
    // for ASIO Driver
    use sysinfo::{Pid, Signal, System};
    let mut sys = System::new_all();
    sys.refresh_all();
    let exec_name = try_get_current_executable_name().unwrap();
    for process in sys.processes_by_exact_name((&exec_name).as_ref()) {
        // debug_println!("[{}] {}\r", process.pid(), process.name());
        if let Some(process) = sys.process(Pid::from(process.pid().as_u32() as usize)) {
            if process.kill_with(Signal::Kill).is_none() {
                eprintln!("This signal isn't supported on this platform");
            }
        }
    }
}

pub fn print_error(error: impl Display) {
    disable_raw_mode().unwrap();
    println!("{} {}", "Error:".bright_red(), error);
}

pub fn print_wran(error: impl Display) {
    disable_raw_mode().unwrap();
    println!("{} {}", "WARN:".bright_yellow(), error);
}

pub struct Quit;
impl Drop for Quit {
    fn drop(&mut self) {
        quit(Error::from(crate::errors::RadicoError::Quit));
    }
}