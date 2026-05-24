#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod haruhi;
mod ui;

fn main() {
    ui::run();
}