#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(failure) = streamlink_twitch_gui_lib::run() {
        streamlink_twitch_gui_lib::startup::report(&failure);
    }
}
