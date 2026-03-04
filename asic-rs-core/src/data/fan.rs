use crate::data::serialize;
use measurements::AngularVelocity;
use serde::{Deserialize, Serialize};
use serialize::serialize_angular_velocity;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct FanData {
    /// The position or index of the fan as seen by the device
    /// Usually dependent on where to fan is connected to the control board
    pub position: i16,
    /// The RPM of the fan
    #[serde(serialize_with = "serialize_angular_velocity")]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rpm: Option<AngularVelocity>,
}
