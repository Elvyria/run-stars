mod terminal;
mod app;
mod render;
mod handler;
mod spinner;

use std::error::Error;

use futures_lite::future::block_on;
use terminal::Terminal;

#[derive(argh::FromArgs)]
/// sad
struct Args {
    /// dir
    #[argh(positional)]
    dir: Option<String>,
}

fn main() -> Result<(), Box<dyn Error>> {
    let args: Args = argh::from_env();

    let mut terminal = Terminal::init().unwrap();
    let res = block_on(app::run(&mut terminal, app::App::new(args.dir)?));

    if let Err(err) = res {
        println!("{err:?}");
    }

    Ok(())
}
