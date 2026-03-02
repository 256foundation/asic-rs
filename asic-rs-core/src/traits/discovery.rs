use crate::data::command::MinerCommand;

pub trait DiscoveryCommands {
    fn get_discovery_commands(&self) -> Vec<MinerCommand>;
}
