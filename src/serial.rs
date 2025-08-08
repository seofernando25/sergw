use std::time::Duration;

use anyhow::{bail, Result};
use serialport::{available_ports, SerialPort, SerialPortBuilder, SerialPortInfo, SerialPortType};

use crate::cli::Listen;

pub fn list_available_ports(include_all: bool) -> Vec<SerialPortInfo> {
    available_ports()
        .unwrap_or_default()
        .into_iter()
        .filter(|port| include_all || matches!(port.port_type, SerialPortType::UsbPort(_)))
        .collect::<Vec<_>>()
}

pub fn select_serial_port(explicit: &Option<String>) -> Result<String> {
    if let Some(p) = explicit {
        return Ok(p.clone());
    }
    let ports = list_available_ports(false)
        .into_iter()
        .map(|p| p.port_name)
        .collect::<Vec<_>>();
    decide_port(None, ports)
}

// Pure decision function for easier testing
pub(crate) fn decide_port(explicit: Option<String>, available: Vec<String>) -> Result<String> {
    if let Some(p) = explicit {
        return Ok(p);
    }
    match available.len() {
        0 => bail!("No serial ports found. Re-run with --serial <PORT> or use --all in 'ports' to inspect."),
        1 => Ok(available[0].clone()),
        _ => bail!(
            "Multiple serial ports detected: {}. Please specify --serial <PORT>.",
            available.join(", ")
        ),
    }
}


pub fn configure_serial(
    builder: SerialPortBuilder,
    listen: &Listen,
) -> serialport::Result<Box<dyn SerialPort>> {
    builder
        .baud_rate(listen.baud)
        .data_bits(listen.data_bits.clone().into())
        .parity(listen.parity.clone().into())
        .stop_bits(listen.stop_bits.clone().into())
        .timeout(Duration::from_millis(200))
        .open()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decide_port_explicit() {
        let r = decide_port(Some("/dev/ttyUSB9".into()), vec!["/dev/ttyUSB0".into()]).unwrap();
        assert_eq!(r, "/dev/ttyUSB9");
    }

    #[test]
    fn test_decide_port_none_single() {
        let r = decide_port(None, vec!["/dev/ttyUSB0".into()]).unwrap();
        assert_eq!(r, "/dev/ttyUSB0");
    }

    #[test]
    fn test_decide_port_none_zero() {
        let err = decide_port(None, vec![]).unwrap_err();
        assert!(err.to_string().contains("No serial ports"));
    }

    #[test]
    fn test_decide_port_none_multiple() {
        let err =
            decide_port(None, vec!["/dev/ttyUSB0".into(), "/dev/ttyUSB1".into()]).unwrap_err();
        assert!(err.to_string().contains("Multiple serial ports"));
    }
}
