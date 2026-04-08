#![feature(iter_intersperse)]
#![feature(string_remove_matches)]
mod app;
mod db_sync;
mod tabs;
mod html_parsing;
mod alerts;

use app::App;

fn main() {
    console_error_panic_hook::set_once();
    yew::Renderer::<App>::new().render();
}
