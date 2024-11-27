use bpaf::*;
use std::fmt::Debug;
use std::io;
use std::path::PathBuf;
use crossterm::{cursor, execute};

const ABOUT: &str = "
A command line music player for Internet Radio.

[Key]                [Description]
 0-9                  adjust volume
 i                    station info
 n                    next station
 p                    previous station
 Q                    quit
 Ctrl+C               exit
";

const USAGE: &str = "
Usage: radico [-s] [--cert=<cert>] [url]

Available positional items:
    url                  url

Available options:
    -s, --show-dev-list  show device list
        --cert=<cert>    certificate
    -h, --help           Prints help information
";

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Options {
    #[bpaf(short, long)]
    /// show device list
    pub show_dev_list: bool,
    #[bpaf(argument("cert"))]
    /// certificate
    pub cert: Option<PathBuf>,
    #[bpaf(any("url", not_help))]
    /// url
    pub url: Option<String>,
}

pub fn about() -> &'static str {
    ABOUT
}
pub fn usage() -> &'static str { USAGE }

fn not_help(s: String) -> Option<String> {
    match s.as_str() {
        "--help" | "-h" => {
            println!("{}", about());
            execute!(io::stdout(), cursor::Show).unwrap();
            None
        },
        _ => Some(s),
    }
}

impl Options {
    pub fn init() -> Options {
        options().run()
    }
}
