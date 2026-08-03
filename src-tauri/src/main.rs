// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
// MSVC's linker prints informational "creating library/object file" notes
// when building the cdylib's import lib - harmless, just noisy.
#![allow(linker_messages)]

fn main() {
    m2manager_lib::run()
}
