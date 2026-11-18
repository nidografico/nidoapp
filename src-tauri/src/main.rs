// Sin consola negra en Windows
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    nido_app_lib::run()
}