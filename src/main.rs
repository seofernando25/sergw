mod cli;
mod serial;
mod server;
mod state;
mod tui;

use anyhow::Result;
use clap::{CommandFactory, Parser};
use tracing_subscriber::EnvFilter;

use crate::cli::{Cli, Commands, PortsFormat};
use crate::serial::list_available_ports;
use crate::server::run_listen;
use serialport::SerialPortType;

fn print_ports(all: bool, verbose: bool, format: PortsFormat) {
    let ports = list_available_ports(all);
    match format {
        PortsFormat::Text => {
            if ports.is_empty() {
                eprintln!("<no ports>");
                std::process::exit(2);
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
        PortsFormat::Json => {
            #[derive(serde::Serialize)]
            struct PortOut {
                name: String,
                kind: String,
                vid: Option<u16>,
                pid: Option<u16>,
                product: Option<String>,
                manufacturer: Option<String>,
            }

            let out: Vec<PortOut> = ports
                .into_iter()
                .map(|p| match p.port_type {
                    SerialPortType::UsbPort(info) => PortOut {
                        name: p.port_name,
                        kind: "usb".into(),
                        vid: Some(info.vid),
                        pid: Some(info.pid),
                        product: info.product,
                        manufacturer: info.manufacturer,
                    },
                    other => PortOut {
                        name: p.port_name,
                        kind: format!("{other:?}"),
                        vid: None,
                        pid: None,
                        product: None,
                        manufacturer: None,
                    },
                })
                .collect();
            println!("{}", serde_json::to_string_pretty(&out).unwrap());
        }
    }
}

fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::from_default_env())
        .with_target(false)
        .try_init()
        .ok();

    let cli = Cli::parse();
    let result: Result<()> = match cli.command {
        Some(Commands::Ports { all, verbose, format }) => {
            print_ports(all, verbose, format);
            Ok(())
        }
        Some(Commands::Listen(listen)) => run_listen(listen),
        None => {
            Cli::command().print_help().ok();
            println!();
            Ok(())
        }
    };

    if let Err(err) = result {
        // Map to stable exit codes
        let code = exit_code_for_error(&err);
        eprintln!("error: {err:?}");
        std::process::exit(code);
    }
}

pub(crate) fn exit_code_for_error(err: &anyhow::Error) -> i32 {
    // 2: no ports, 3: multiple ports, 4: bind failure, 5: serial open failure, 1: other
    for cause in err.chain() {
        if let Some(sel) = cause.downcast_ref::<crate::serial::SerialSelectError>() {
            return match sel {
                crate::serial::SerialSelectError::NoPorts => 2,
                crate::serial::SerialSelectError::MultiplePorts { .. } => 3,
            };
        }
        if let Some(ioe) = cause.downcast_ref::<std::io::Error>() {
            use std::io::ErrorKind::*;
            return match ioe.kind() {
                AddrInUse | AddrNotAvailable | PermissionDenied | ConnectionAborted | ConnectionReset => 4,
                _ => 1,
            };
        }
        if cause.is::<serialport::Error>() {
            return 5;
        }
    }
    1
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exit_code_no_ports() {
        let err = anyhow::Error::from(crate::serial::SerialSelectError::NoPorts);
        assert_eq!(exit_code_for_error(&err), 2);
    }

    #[test]
    fn exit_code_multiple_ports() {
        let err = anyhow::Error::from(crate::serial::SerialSelectError::MultiplePorts { list: vec!["a".into(), "b".into()] });
        assert_eq!(exit_code_for_error(&err), 3);
    }

    #[test]
    fn exit_code_bind_like_io_error() {
        let err = anyhow::Error::from(std::io::Error::from(std::io::ErrorKind::AddrInUse));
        assert_eq!(exit_code_for_error(&err), 4);
    }

    #[test]
    fn exit_code_serial_error() {
        let serr = serialport::Error::new(serialport::ErrorKind::NoDevice, "no device");
        let err = anyhow::Error::from(serr);
        assert_eq!(exit_code_for_error(&err), 5);
    }

    #[test]
    fn exit_code_other() {
        let err = anyhow::anyhow!("other");
        assert_eq!(exit_code_for_error(&err), 1);
    }
}
