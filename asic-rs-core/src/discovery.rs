use crate::data::command::DiscoveryCommand;

pub const DEFAULT_RPC_PORT: u16 = 4028;
pub const DEFAULT_WEB_PORT: u16 = 80;

pub const RPC_DEVDETAILS: DiscoveryCommand = DiscoveryCommand::RPC {
    command: "devdetails",
    port: DEFAULT_RPC_PORT,
};
pub const RPC_VERSION: DiscoveryCommand = DiscoveryCommand::RPC {
    command: "version",
    port: DEFAULT_RPC_PORT,
};
pub const HTTP_WEB_ROOT: DiscoveryCommand = DiscoveryCommand::Web {
    command: "/",
    port: DEFAULT_WEB_PORT,
};
