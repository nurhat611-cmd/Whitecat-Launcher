// Чтобы в release-сборке под Windows не появлялась консоль.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    lighty_launcher_lib::run()
}
