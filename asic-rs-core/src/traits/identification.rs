/// Structured web response data for firmware identification.
pub struct WebResponse<'a> {
    pub body: &'a str,
    pub auth_header: &'a str,
    pub algo_header: &'a str,
    pub redirect_header: &'a str,
    pub status: u16,
}

/// Trait for identifying firmware from discovery responses.
pub trait FirmwareIdentification {
    fn identify_rpc(&self, _response: &str) -> bool {
        false
    }
    fn identify_web(&self, _response: &WebResponse<'_>) -> bool {
        false
    }
    /// Returns true if this is a stock firmware that may be superseded
    /// by a third-party firmware running on the same device.
    fn is_stock(&self) -> bool {
        false
    }
}
