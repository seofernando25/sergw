mod cli;
mod serial;
mod server;
mod state;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Commands};
use crate::serial::list_available_ports;
use crate::server::run_listen;
use serialport::SerialPortType;

fn print_ports(all: bool, verbose: bool) {
    let ports = list_available_ports(all);
    if ports.is_empty() {
        println!("<no ports>");
        return;
    }
    for p in ports {
        if verbose {
            match p.port_type {
                SerialPortType::UsbPort(info) => {
                    println!(
                        "{}\tUSB vid:pid {:04x}:{:04x}\t{:?}\t{:?}",
                        p.port_name, info.vid, info.pid, info.product, info.manufacturer,
                    );
                }
                other => {
                    println!("{}\t{:?}", p.port_name, other);
                }
            }
        } else {
            println!("{}", p.port_name);
        }
    }
}

fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .try_init()
        .ok();

    let cli = Cli::parse();
    match cli.command {
        Some(Commands::Ports { all, verbose }) => {
            print_ports(all, verbose);
            Ok(())
        }
        Some(Commands::Listen(listen)) => run_listen(listen),
        None => {
            Cli::command().print_help().ok();
            println!();
            Ok(())
        }
    }
}
