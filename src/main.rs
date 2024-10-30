mod api;
mod args;
mod errors;
mod macros;
mod player;
mod sink;
mod sleep;
mod state;
mod stream;
mod terminal;
mod worker;
mod xml;

use crate::worker::main_thread;

#[tokio::main(flavor = "multi_thread", worker_threads = 1)]
async fn main() {
    args::init();
    terminal::init();
    if let Err(e) = main_thread().await {
        terminal::quit(e);
    }
    let _exit = terminal::Quit;
}
