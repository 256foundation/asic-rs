use crate::data::command::DiscoveryCommand;

pub trait DiscoveryCommands {
    fn get_discovery_commands(&self) -> Vec<DiscoveryCommand>;
}
