use clap::Parser;
use std::fmt::Debug;
use std::sync::Mutex;

const ABOUT: &str = "
A command line music player for Internet Radio.

[Key]                [Description]
 0-9                  adjust volume
 i                    station info
 n                    next station
 p                    previous station
 Q                    quit
 Ctrl+C               exit";

#[derive(Parser, Debug, Clone)]
#[clap(author, version, about = ABOUT)]
pub struct Args {
    /// url
    pub url: String,
}

#[allow(unused)]
pub fn about() -> &'static str {
    ABOUT
}

static ARGS: Mutex<Option<Args>> = Mutex::new(None);

pub fn init() {
    let args: Args = Args::parse();
    ARGS.lock().unwrap().replace(args);
}

pub fn args_url() -> String {
    ARGS.lock().unwrap().as_ref().unwrap().url.to_owned()
}
