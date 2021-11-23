// LNP Node: node running lightning network protocol and generalized lightning
// channels.
// Written in 2020 by
//     Dr. Maxim Orlovsky <orlovsky@pandoracore.com>
//
// To the extent possible under law, the author(s) have dedicated all
// copyright and related and neighboring rights to this software to
// the public domain worldwide. This software is distributed without
// any warranty.
//
// You should have received a copy of the MIT License
// along with this software.
// If not, see <https://opensource.org/licenses/MIT>.

use clap::{Clap, ValueHint};
use std::fs;
use std::net::SocketAddr;
use std::path::PathBuf;

use internet2::PartialNodeAddr;
use lnpbp::chain::Chain;
use microservices::shell::LogLevel;

#[cfg(any(target_os = "linux"))]
pub const FARCASTER_DATA_DIR: &str = "~/.farcaster";
#[cfg(any(target_os = "freebsd", target_os = "openbsd", target_os = "netbsd"))]
pub const FARCASTER_DATA_DIR: &str = "~/.farcaster";
#[cfg(target_os = "macos")]
pub const FARCASTER_DATA_DIR: &str = "~/Library/Application Support/Farcaster";
#[cfg(target_os = "windows")]
pub const FARCASTER_DATA_DIR: &str = "~\\AppData\\Local\\Farcaster";
#[cfg(target_os = "ios")]
pub const FARCASTER_DATA_DIR: &str = "~/Documents";
#[cfg(target_os = "android")]
pub const FARCASTER_DATA_DIR: &str = ".";

pub const FARCASTER_MSG_SOCKET_NAME: &str = "lnpz:{data_dir}/msg.rpc?api=esb";
pub const FARCASTER_CTL_SOCKET_NAME: &str = "lnpz:{data_dir}/ctl.rpc?api=esb";

pub const FARCASTER_CONFIG: &str = "{data_dir}/farcaster.toml";

pub const FARCASTER_TOR_PROXY: &str = "127.0.0.1:9050";
pub const FARCASTER_KEY_FILE: &str = "{data_dir}/key.dat";

pub const FARCASTER_ELECTRUM_SERVER: &str = "ssl://blockstream.info:700";
pub const FARCASTER_MONERO_DAEMON: &str = "http://node.monerooutreach.org:18081";
pub const FARCASTER_MONERO_RPC_WALLET: &str = "http://localhost:18083";

/// Shared options used by different binaries
#[derive(Clap, Clone, PartialEq, Eq, Debug)]
pub struct Opts {
    /// Data directory path
    ///
    /// Path to the directory that contains FARCASTER Node data, and where ZMQ
    /// RPC socket files are located
    #[clap(
        short,
        long,
        global = true,
        default_value = FARCASTER_DATA_DIR,
        env = "FARCASTER_DATA_DIR",
        value_hint = ValueHint::DirPath
    )]
    pub data_dir: PathBuf,

    /// Path to the configuration file.
    ///
    /// NB: Command-line options override configuration file values.
    #[clap(
        short,
        long,
        global = true,
        env = "FARCASTER_CONFIG",
        value_hint = ValueHint::FilePath
    )]
    pub config: Option<String>,

    /// Set verbosity level
    ///
    /// Can be used multiple times to increase verbosity
    #[clap(short, long, global = true, parse(from_occurrences))]
    pub verbose: u8,

    /// Use Tor
    ///
    /// If set, specifies SOCKS5 proxy used for Tor connectivity and directs
    /// all network traffic through Tor network.
    /// If the argument is provided in form of flag, without value, uses
    /// `127.0.0.1:9050` as default Tor proxy address.
    #[clap(
        short = 'T',
        long,
        alias = "tor",
        global = true,
        env = "FARCASTER_TOR_PROXY",
        value_hint = ValueHint::Hostname
    )]
    pub tor_proxy: Option<Option<SocketAddr>>,

    /// ZMQ socket name/address to forward all incoming lightning messages
    ///
    /// Internal interface for transmitting P2P lightning network messages.
    /// Defaults to `msg.rpc` file inside `--data-dir` directory, unless
    /// `--use-threads` is specified; in that cases uses in-memory
    /// communication protocol.
    #[clap(
        short = 'm',
        long,
        global = true,
        env = "FARCASTER_MSG_SOCKET",
        value_hint = ValueHint::FilePath,
        default_value = FARCASTER_MSG_SOCKET_NAME
    )]
    pub msg_socket: PartialNodeAddr,

    /// ZMQ socket name/address for daemon control interface
    ///
    /// Internal interface for control PRC protocol communications
    /// Defaults to `ctl.rpc` file inside `--data-dir` directory, unless
    /// `--use-threads` is specified; in that cases uses in-memory
    /// communication protocol.
    #[clap(
        short = 'x',
        long,
        global = true,
        env = "FARCASTER_CTL_SOCKET",
        value_hint = ValueHint::FilePath,
        default_value = FARCASTER_CTL_SOCKET_NAME
    )]
    pub ctl_socket: PartialNodeAddr,

    /// Blockchain to use
    #[clap(
        short = 'n',
        long,
        global = true,
        alias = "network",
        default_value = "testnet",
        env = "FARCASTER_NETWORK"
    )]
    // TODO: Put it back to `signet` default network once rust-bitcoin will
    //       release signet support
    pub chain: Chain,

    /// Electrum server to use
    #[clap(
        long,
        global = true,
        default_value = FARCASTER_ELECTRUM_SERVER,
        env = "FARCASTER_ELECTRUM_SERVER"
    )]
    pub electrum_server: String,

    /// Monero daemon to use
    #[clap(
        long,
        global = true,
        default_value = FARCASTER_MONERO_DAEMON,
        env = "FARCASTER_MONERO_DAEMON"
    )]
    pub monero_daemon: String,

    /// Monero rpc wallet to use
    #[clap(
        long,
        global = true,
        default_value = FARCASTER_MONERO_RPC_WALLET,
        env = "FARCASTER_MONERO_RPC_WALLET"
    )]
    pub monero_rpc_wallet: String,
}

impl Opts {
    pub fn process(&mut self) {
        LogLevel::from_verbosity_flag_count(self.verbose).apply();
        let mut me = self.clone();

        me.data_dir = PathBuf::from(
            shellexpand::tilde(&me.data_dir.to_string_lossy().to_string()).to_string(),
        );
        fs::create_dir_all(&me.data_dir).expect("Unable to access data directory");

        for s in vec![&mut self.msg_socket, &mut self.ctl_socket] {
            match s {
                PartialNodeAddr::ZmqIpc(path, ..) | PartialNodeAddr::Posix(path) => {
                    me.process_dir(path);
                }
                _ => {}
            }
        }
    }

    pub fn process_dir(&self, path: &mut String) {
        *path = path.replace("{data_dir}", &self.data_dir.to_string_lossy());
        *path = shellexpand::tilde(path).to_string();
    }
}
