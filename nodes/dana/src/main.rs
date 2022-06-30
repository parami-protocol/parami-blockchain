//! Substrate Node Template CLI library.
#![warn(missing_docs)]

mod chain_spec;
#[macro_use]
mod service;
mod cli;
mod command;
mod command_helper;
mod rpc;

#[cfg(not(target_os = "windows"))]
fn main() -> sc_cli::Result<()> {
    command::run()
}

#[cfg(target_os = "windows")]
fn main() -> sc_cli::Result<()> {
    let child = std::thread::Builder::new()
        .stack_size(32 * 1024 * 1024)
        .spawn(move || -> sc_cli::Result<()> { command::run() })
        .unwrap();

    child.join().unwrap()
}
