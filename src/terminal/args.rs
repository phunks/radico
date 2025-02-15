use bpaf::{Bpaf, Parser, short};
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
Usage: radico [-s] [--cert=<cert>] [--proxy=<socks>] [url]

Available positional items:
    url                  url

Available options:
    -s, --show-dev-list  show device list
        --cert=<cert>    certificate
        --proxy=<socks>  ex: [https|socks5]://<ip>:<port>
    -h, --help           Prints help information
";

#[derive(Debug, Clone, Bpaf)]
#[bpaf(options)]
pub struct Options {
    #[bpaf(external(verbose))]
    /// verbose log
    pub verbose: usize,
    #[bpaf(short, long)]
    /// show device list
    pub show_dev_list: bool,
    #[bpaf(argument("cert"))]
    /// certificate
    pub cert: Option<PathBuf>,
    #[bpaf(argument("proxy"))]
    /// ex: [http(s)|socks5]://<ip>:<port>
    pub proxy: Option<String>,
    #[bpaf(any("url", not_help))]
    /// url
    pub url: Option<String>,
}

fn verbose() -> impl Parser<usize> {
    // number of occurrences of the v/verbose flag capped at 3
    short('v')
        .long("verbose")
        .help("Increase the verbosity\nYou can specify it up to 3 times\neither as -v -v -v or as -vvv")
        .req_flag(())
        .many()
        .map(|xs| xs.len())
        .guard(|&x| x <= 3, "It doesn't get any more verbose than this")
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
