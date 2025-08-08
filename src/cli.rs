use std::net::SocketAddr;

use clap::{Parser, Subcommand, ValueEnum};
use serialport::{DataBits, Parity, StopBits};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
#[command(propagate_version = true)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// List available serial ports
    Ports {
        /// Include non-USB ports as well
        #[arg(long)]
        all: bool,
        /// Show detailed metadata
        #[arg(long)]
        verbose: bool,
    },
    /// Bridge a serial port to TCP
    Listen(Listen),
}

#[derive(Parser, Clone, Debug)]
pub struct Listen {
    /// Serial port to open (auto-select if exactly one is found and this is omitted)
    #[arg(long)]
    pub serial: Option<String>,

    /// Baud rate
    #[arg(long, default_value_t = 115_200)]
    pub baud: u32,

    /// TCP listen address
    #[arg(long, default_value = "127.0.0.1:5656")]
    pub host: SocketAddr,

    /// Data bits
    #[arg(long, value_enum, default_value_t = DataBitsOpt::Eight)]
    pub data_bits: DataBitsOpt,

    /// Parity
    #[arg(long, value_enum, default_value_t = ParityOpt::None)]
    pub parity: ParityOpt,

    /// Stop bits
    #[arg(long, value_enum, default_value_t = StopBitsOpt::One)]
    pub stop_bits: StopBitsOpt,
}

#[derive(ValueEnum, Clone, Debug)]
pub enum DataBitsOpt {
    Five,
    Six,
    Seven,
    Eight,
}

impl From<DataBitsOpt> for DataBits {
    fn from(v: DataBitsOpt) -> Self {
        match v {
            DataBitsOpt::Five => DataBits::Five,
            DataBitsOpt::Six => DataBits::Six,
            DataBitsOpt::Seven => DataBits::Seven,
            DataBitsOpt::Eight => DataBits::Eight,
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum ParityOpt {
    None,
    Odd,
    Even,
}

impl From<ParityOpt> for Parity {
    fn from(v: ParityOpt) -> Self {
        match v {
            ParityOpt::None => Parity::None,
            ParityOpt::Odd => Parity::Odd,
            ParityOpt::Even => Parity::Even,
        }
    }
}

#[derive(ValueEnum, Clone, Debug)]
pub enum StopBitsOpt {
    One,
    Two,
}

impl From<StopBitsOpt> for StopBits {
    fn from(v: StopBitsOpt) -> Self {
        match v {
            StopBitsOpt::One => StopBits::One,
            StopBitsOpt::Two => StopBits::Two,
        }
    }
}
