use crate::data::command::MinerCommand;

pub const RPC_DEVDETAILS: MinerCommand = MinerCommand::RPC {
    command: "devdetails",
    parameters: None,
};
pub const RPC_VERSION: MinerCommand = MinerCommand::RPC {
    command: "version",
    parameters: None,
};
pub const HTTP_WEB_ROOT: MinerCommand = MinerCommand::WebAPI {
    command: "/",
    parameters: None,
};
