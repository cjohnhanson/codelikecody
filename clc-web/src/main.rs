#[allow(dead_code)]
mod api;
mod app;
mod components;
mod pages;
#[allow(dead_code)]
mod types;

fn main() {
    console_error_panic_hook::set_once();
    leptos::mount::mount_to_body(app::App);
}
