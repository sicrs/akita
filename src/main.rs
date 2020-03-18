mod app;
mod dto;
mod url;

use app::{init, AkitaClient};
use cli::App;
use std::env::args;

fn main() {
    let app: App<AkitaClient> = init();
    let mut args: Vec<String> = args().collect();
    if args[0].contains("akita") {
        args.remove(0);
    }
    app.run(args);
}
