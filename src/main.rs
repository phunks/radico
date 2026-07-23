use crate::api::worker::Queue;
#[allow(unused_imports)]
use crate::logger::Logger;

mod api;
mod audio;
mod errors;
mod terminal;
mod util;
mod logger;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let _exit = terminal::Quit;
    terminal::init();
    // let _logger = Logger::build(2);
    let mut m = Queue::default();

    if let Err(e) = m.worker().await {
        terminal::quit(e);
    }
}
