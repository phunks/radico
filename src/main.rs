use crate::api::worker::Queue;

mod api;
mod audio;
mod errors;
mod terminal;
mod util;

#[tokio::main(flavor = "multi_thread", worker_threads = 2)]
async fn main() {
    let _exit = terminal::Quit;
    terminal::init();
    let mut m = Queue::default();

    if let Err(e) = m.worker().await {
        terminal::quit(e);
    }
}
