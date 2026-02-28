#![feature(iter_intersperse)]
mod app;
mod db_sync;
mod tabs;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
