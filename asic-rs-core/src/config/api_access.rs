//! RPC API access control for cgminer-family firmware.
//!
//! cgminer, bmminer and the firmware built on them gate their TCP RPC API
//! (port 4028) with four settings:
//!
//! - `api-listen`: whether the API listens at all.
//! - `api-network`: accept connections from other hosts, not only localhost.
//!   With no `api-allow` list every allowed host gets read-only access.
//! - `api-groups`: extra command groups, e.g. `A:stats:pools:devs:summary:version`.
//! - `api-allow`: a comma separated list of `[G:]address[/prefix]` rules. It
//!   overrides `api-network`. `W:` grants privileged access (every command),
//!   `R:` or no prefix grants read-only access, and any other letter refers to
//!   a group from `api-groups`. `0/0` matches every address. The first rule that
//!   matches a client decides its access, so a `W:` rule must come before a
//!   catch-all such as `A:0/0`.
//!
//! [`ApiAllowList`] parses and writes the `api-allow` string with the same
//! rules cgminer applies in `setup_ipaccess`, so a list read from a miner can be
//! inspected, changed and written back without losing entries cgminer ignores.

use std::{
    fmt::{self, Display},
    net::{IpAddr, Ipv4Addr, Ipv6Addr},
    str::FromStr,
};

use serde::{Deserialize, Deserializer, Serialize, Serializer};

/// Access group letter cgminer uses for privileged (write) access.
pub const WRITE_GROUP: char = 'W';
/// Access group letter cgminer uses for read-only access.
pub const READ_GROUP: char = 'R';

/// An address or subnet an `api-allow` rule applies to.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ApiAccessTarget {
    /// `0/0`: every IPv4 and IPv6 client.
    Any,
    /// A single address or a subnet in CIDR form.
    Network(ApiAccessNetwork),
    /// An entry cgminer cannot parse (for example the `*` in the stock
    /// Antminer default `A:0/0,W:*`). cgminer skips it; it is kept verbatim so
    /// a list round-trips unchanged.
    Unparsed(String),
}

/// An IP address with a prefix length, as accepted by `api-allow`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ApiAccessNetwork {
    addr: IpAddr,
    prefix: u8,
}

impl ApiAccessNetwork {
    /// Create a network from an address and prefix length.
    ///
    /// The prefix must be 1-32 for IPv4 and 1-128 for IPv6; cgminer ignores a
    /// zero prefix other than the `0/0` catch-all ([`ApiAccessTarget::Any`]).
    pub fn new(addr: IpAddr, prefix: u8) -> anyhow::Result<Self> {
        let max = max_prefix(&addr);
        if prefix == 0 || prefix > max {
            anyhow::bail!("invalid prefix /{prefix} for {addr}, expected 1-{max}");
        }
        Ok(Self { addr, prefix })
    }

    /// A single host (`/32` for IPv4, `/128` for IPv6).
    pub fn host(addr: IpAddr) -> Self {
        Self {
            prefix: max_prefix(&addr),
            addr,
        }
    }

    /// The configured address.
    pub fn addr(&self) -> IpAddr {
        self.addr
    }

    /// The prefix length.
    pub fn prefix(&self) -> u8 {
        self.prefix
    }

    /// Whether this rule names one host.
    pub fn is_host(&self) -> bool {
        self.prefix == max_prefix(&self.addr)
    }

    /// The same network with the host bits cleared.
    ///
    /// cgminer compares `client & mask` against the address as written, so a
    /// rule such as `W:192.0.2.5/24` never matches anything. Use this when
    /// building rules.
    pub fn truncated(&self) -> Self {
        let addr = match self.addr {
            IpAddr::V4(v4) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                IpAddr::V4(Ipv4Addr::from(u32::from(v4) & mask))
            }
            IpAddr::V6(v6) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                IpAddr::V6(Ipv6Addr::from(u128::from(v6) & mask))
            }
        };
        Self {
            addr,
            prefix: self.prefix,
        }
    }

    /// Whether cgminer would match `client` against this rule.
    pub fn matches(&self, client: IpAddr) -> bool {
        match (self.addr, client) {
            (IpAddr::V4(rule), IpAddr::V4(client)) => {
                let mask = u32::MAX
                    .checked_shl(32 - u32::from(self.prefix))
                    .unwrap_or(0);
                u32::from(client) & mask == u32::from(rule)
            }
            (IpAddr::V6(rule), IpAddr::V6(client)) => {
                let mask = u128::MAX
                    .checked_shl(128 - u32::from(self.prefix))
                    .unwrap_or(0);
                u128::from(client) & mask == u128::from(rule)
            }
            _ => false,
        }
    }
}

impl From<IpAddr> for ApiAccessNetwork {
    fn from(addr: IpAddr) -> Self {
        Self::host(addr)
    }
}

impl FromStr for ApiAccessNetwork {
    type Err = anyhow::Error;

    /// Parse `address[/prefix]` the way cgminer does: IPv4 may be shortened
    /// (`198.51.100/24` is `198.51.100.0/24`) and IPv6 must be bracketed (`[2001:db8::]/32`).
    /// A bare IPv6 address without brackets is accepted as well.
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let (addr, prefix) = match s.split_once('/') {
            Some((addr, prefix)) => (addr, Some(prefix)),
            None => (s, None),
        };

        let addr = if let Some(inner) = addr.strip_prefix('[').and_then(|a| a.strip_suffix(']')) {
            IpAddr::V6(inner.parse::<Ipv6Addr>()?)
        } else if addr.contains(':') {
            IpAddr::V6(addr.parse::<Ipv6Addr>()?)
        } else {
            IpAddr::V4(parse_padded_ipv4(addr)?)
        };

        match prefix {
            None => Ok(Self::host(addr)),
            Some(prefix) => {
                let prefix = prefix
                    .parse::<u8>()
                    .map_err(|_| anyhow::anyhow!("invalid prefix '/{prefix}'"))?;
                Self::new(addr, prefix)
            }
        }
    }
}

impl Display for ApiAccessNetwork {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.addr {
            IpAddr::V4(v4) => write!(f, "{v4}")?,
            IpAddr::V6(v6) => write!(f, "[{v6}]")?,
        }
        if !self.is_host() {
            write!(f, "/{}", self.prefix)?;
        }
        Ok(())
    }
}

fn max_prefix(addr: &IpAddr) -> u8 {
    match addr {
        IpAddr::V4(_) => 32,
        IpAddr::V6(_) => 128,
    }
}

/// cgminer pads a short IPv4 address with `.0` octets (`198.51.100` is `198.51.100.0`).
fn parse_padded_ipv4(addr: &str) -> anyhow::Result<Ipv4Addr> {
    let octets = addr.split('.').count();
    if addr.is_empty() || octets > 4 {
        anyhow::bail!("invalid IPv4 address '{addr}'");
    }
    let padded = format!("{addr}{}", ".0".repeat(4 - octets));
    padded
        .parse::<Ipv4Addr>()
        .map_err(|_| anyhow::anyhow!("invalid IPv4 address '{addr}'"))
}

impl FromStr for ApiAccessTarget {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        if s == "0/0" {
            return Ok(Self::Any);
        }
        Ok(match s.parse::<ApiAccessNetwork>() {
            Ok(network) => Self::Network(network),
            Err(_) => Self::Unparsed(s.to_string()),
        })
    }
}

impl Display for ApiAccessTarget {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Any => f.write_str("0/0"),
            Self::Network(network) => network.fmt(f),
            Self::Unparsed(raw) => f.write_str(raw),
        }
    }
}

impl From<ApiAccessNetwork> for ApiAccessTarget {
    fn from(network: ApiAccessNetwork) -> Self {
        Self::Network(network)
    }
}

impl From<IpAddr> for ApiAccessTarget {
    fn from(addr: IpAddr) -> Self {
        Self::Network(ApiAccessNetwork::host(addr))
    }
}

/// One `api-allow` entry: an optional group letter and a target.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ApiAccessRule {
    /// Group letter as written (upper-cased), or `None` for a bare address,
    /// which cgminer treats as read-only.
    pub group: Option<char>,
    /// The address, subnet or catch-all this rule applies to.
    pub target: ApiAccessTarget,
}

impl ApiAccessRule {
    /// Privileged (`W:`) access for a target.
    pub fn write(target: impl Into<ApiAccessTarget>) -> Self {
        Self {
            group: Some(WRITE_GROUP),
            target: target.into(),
        }
    }

    /// Read-only (`R:`) access for a target.
    pub fn read(target: impl Into<ApiAccessTarget>) -> Self {
        Self {
            group: Some(READ_GROUP),
            target: target.into(),
        }
    }

    /// The effective group letter: a bare address is read-only (`R`).
    pub fn effective_group(&self) -> char {
        self.group.unwrap_or(READ_GROUP)
    }

    /// Whether this rule grants privileged access.
    pub fn is_write(&self) -> bool {
        self.effective_group() == WRITE_GROUP
    }

    /// Whether cgminer would match `client` against this rule.
    pub fn matches(&self, client: IpAddr) -> bool {
        match &self.target {
            ApiAccessTarget::Any => true,
            ApiAccessTarget::Network(network) => network.matches(client),
            ApiAccessTarget::Unparsed(_) => false,
        }
    }
}

impl FromStr for ApiAccessRule {
    type Err = std::convert::Infallible;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let s = s.trim();
        let mut chars = s.chars();
        let (group, rest) = match (chars.next(), chars.next()) {
            (Some(letter), Some(':')) if letter.is_ascii_alphabetic() => {
                (Some(letter.to_ascii_uppercase()), &s[2..])
            }
            _ => (None, s),
        };
        Ok(Self {
            group,
            target: rest.parse()?,
        })
    }
}

impl Display for ApiAccessRule {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(group) = self.group {
            write!(f, "{group}:")?;
        }
        self.target.fmt(f)
    }
}

/// The ordered `api-allow` list. The first matching rule wins.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct ApiAllowList {
    rules: Vec<ApiAccessRule>,
}

impl ApiAllowList {
    pub fn new(rules: Vec<ApiAccessRule>) -> Self {
        Self { rules }
    }

    pub fn rules(&self) -> &[ApiAccessRule] {
        &self.rules
    }

    pub fn into_rules(self) -> Vec<ApiAccessRule> {
        self.rules
    }

    pub fn is_empty(&self) -> bool {
        self.rules.is_empty()
    }

    /// The rule cgminer applies to `client`, or `None` when the client is
    /// refused.
    pub fn rule_for(&self, client: IpAddr) -> Option<&ApiAccessRule> {
        self.rules.iter().find(|rule| rule.matches(client))
    }

    /// Whether `client` gets privileged access.
    pub fn allows_write(&self, client: IpAddr) -> bool {
        self.rule_for(client).is_some_and(ApiAccessRule::is_write)
    }

    /// Give `target` privileged access.
    ///
    /// The `W:` rule goes first so a catch-all later in the list cannot
    /// shadow it, and other rules for exactly the same target are dropped.
    /// Subnets are truncated to their network address, because cgminer never
    /// matches a rule with host bits set.
    pub fn grant_write(&mut self, target: impl Into<ApiAccessTarget>) {
        let target = normalize_target(target.into());
        self.rules.retain(|rule| rule.target != target);
        self.rules.insert(0, ApiAccessRule::write(target));
    }

    /// Remove every rule for exactly `target`.
    pub fn remove(&mut self, target: impl Into<ApiAccessTarget>) {
        let target = normalize_target(target.into());
        self.rules.retain(|rule| rule.target != target);
    }
}

fn normalize_target(target: ApiAccessTarget) -> ApiAccessTarget {
    match target {
        ApiAccessTarget::Network(network) => ApiAccessTarget::Network(network.truncated()),
        other => other,
    }
}

impl FromStr for ApiAllowList {
    type Err = std::convert::Infallible;

    /// Parse an `api-allow` string. Empty entries are skipped, as cgminer does;
    /// entries cgminer cannot parse are kept as [`ApiAccessTarget::Unparsed`].
    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let rules = s
            .split(',')
            .map(str::trim)
            .filter(|entry| !entry.is_empty())
            .map(|entry| entry.parse())
            .collect::<Result<Vec<_>, _>>()?;
        Ok(Self { rules })
    }
}

impl Display for ApiAllowList {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, rule) in self.rules.iter().enumerate() {
            if i > 0 {
                f.write_str(",")?;
            }
            rule.fmt(f)?;
        }
        Ok(())
    }
}

impl Serialize for ApiAllowList {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for ApiAllowList {
    fn deserialize<D: Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let raw = String::deserialize(deserializer)?;
        raw.parse().map_err(serde::de::Error::custom)
    }
}

/// A named command group from `api-groups`, such as
/// `A:stats:pools:devs:summary:version`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ApiCommandGroup {
    /// Group letter (upper-cased). `R` and `W` are reserved by cgminer.
    pub letter: char,
    /// Commands the group may run. `*` means every read-only command.
    pub commands: Vec<String>,
}

impl ApiCommandGroup {
    /// Parse one `api-groups` entry.
    pub fn parse(entry: &str) -> anyhow::Result<Self> {
        let mut parts = entry.trim().split(':');
        let letter = parts
            .next()
            .and_then(|letter| {
                let mut chars = letter.chars();
                match (chars.next(), chars.next()) {
                    (Some(c), None) if c.is_ascii_alphabetic() => Some(c.to_ascii_uppercase()),
                    _ => None,
                }
            })
            .ok_or_else(|| anyhow::anyhow!("invalid API group '{entry}'"))?;
        let commands = parts
            .filter(|command| !command.is_empty())
            .map(str::to_string)
            .collect();
        Ok(Self { letter, commands })
    }

    /// Parse a whole `api-groups` string; invalid entries are an error.
    pub fn parse_list(groups: &str) -> anyhow::Result<Vec<Self>> {
        groups
            .split(',')
            .filter(|entry| !entry.trim().is_empty())
            .map(Self::parse)
            .collect()
    }

    /// Write a list of groups back to the `api-groups` form.
    pub fn format_list(groups: &[Self]) -> String {
        groups
            .iter()
            .map(ToString::to_string)
            .collect::<Vec<_>>()
            .join(",")
    }
}

impl Display for ApiCommandGroup {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.letter)?;
        for command in &self.commands {
            write!(f, ":{command}")?;
        }
        Ok(())
    }
}

fn serialize_groups<S: Serializer>(
    groups: &Option<Vec<ApiCommandGroup>>,
    serializer: S,
) -> Result<S::Ok, S::Error> {
    match groups {
        Some(groups) => serializer.serialize_some(&ApiCommandGroup::format_list(groups)),
        None => serializer.serialize_none(),
    }
}

fn deserialize_groups<'de, D: Deserializer<'de>>(
    deserializer: D,
) -> Result<Option<Vec<ApiCommandGroup>>, D::Error> {
    Option::<String>::deserialize(deserializer)?
        .map(|groups| ApiCommandGroup::parse_list(&groups).map_err(serde::de::Error::custom))
        .transpose()
}

/// RPC API access settings of a cgminer-family miner.
///
/// Each field is `None` when the firmware does not report it.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApiAccessConfig {
    /// `api-listen`: whether the RPC API is enabled.
    pub listen: Option<bool>,
    /// `api-network`: whether hosts other than localhost may connect. Ignored
    /// by cgminer when an allow list is set.
    pub network: Option<bool>,
    /// `api-groups`: extra command groups referenced by the allow list.
    #[serde(
        default,
        serialize_with = "serialize_groups",
        deserialize_with = "deserialize_groups"
    )]
    pub groups: Option<Vec<ApiCommandGroup>>,
    /// `api-allow`: the ordered access rules.
    pub allow: Option<ApiAllowList>,
}

impl ApiAccessConfig {
    /// Whether `client` may run privileged commands such as `restart`.
    ///
    /// Without an allow list cgminer gives no host privileged access.
    pub fn allows_write(&self, client: IpAddr) -> bool {
        self.listen != Some(false)
            && self
                .allow
                .as_ref()
                .is_some_and(|allow| allow.allows_write(client))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ip(s: &str) -> IpAddr {
        s.parse().unwrap()
    }

    #[test]
    fn stock_antminer_default_round_trips_and_grants_no_write() {
        // The list stock Antminer firmware writes into its miner config.
        let allow: ApiAllowList = "A:0/0,W:*".parse().unwrap();
        assert_eq!(
            allow.rules(),
            &[
                ApiAccessRule {
                    group: Some('A'),
                    target: ApiAccessTarget::Any,
                },
                ApiAccessRule {
                    group: Some('W'),
                    target: ApiAccessTarget::Unparsed("*".into()),
                },
            ]
        );
        assert_eq!(allow.to_string(), "A:0/0,W:*");
        // A:0/0 matches everyone first; `*` is not an address cgminer accepts.
        assert_eq!(allow.rule_for(ip("192.0.2.10")).unwrap().group, Some('A'));
        assert!(!allow.allows_write(ip("192.0.2.10")));
        assert!(!allow.allows_write(ip("127.0.0.1")));
    }

    #[test]
    fn parses_hosts_subnets_and_padded_ipv4() {
        let allow: ApiAllowList =
            " W:192.0.2.1, 198.51.100/24 ,R:203.0.113/24,,W:[2001:db8::1],P:[2001:db8::]/32"
                .parse()
                .unwrap();
        let targets: Vec<String> = allow.rules().iter().map(ToString::to_string).collect();
        assert_eq!(
            targets,
            [
                "W:192.0.2.1",
                "198.51.100.0/24",
                "R:203.0.113.0/24",
                "W:[2001:db8::1]",
                "P:[2001:db8::]/32"
            ]
        );
        assert!(allow.allows_write(ip("192.0.2.1")));
        assert!(!allow.allows_write(ip("192.0.2.2")));
        // A bare address is read-only.
        assert_eq!(
            allow
                .rule_for(ip("198.51.100.77"))
                .unwrap()
                .effective_group(),
            'R'
        );
        assert!(allow.rule_for(ip("203.0.113.200")).is_some());
        assert!(allow.rule_for(ip("192.0.2.200")).is_none());
        assert!(allow.allows_write(ip("2001:db8::1")));
        assert_eq!(
            allow.rule_for(ip("2001:db8:1::5")).unwrap().group,
            Some('P')
        );
    }

    #[test]
    fn invalid_entries_are_kept_but_never_match() {
        let allow: ApiAllowList = "W:192.0.2.0/0,W:999.1.1.1,W:192.0.2.1/33,W:host"
            .parse()
            .unwrap();
        assert!(
            allow
                .rules()
                .iter()
                .all(|rule| matches!(rule.target, ApiAccessTarget::Unparsed(_)))
        );
        assert_eq!(
            allow.to_string(),
            "W:192.0.2.0/0,W:999.1.1.1,W:192.0.2.1/33,W:host"
        );
        assert!(!allow.allows_write(ip("192.0.2.1")));
    }

    #[test]
    fn rule_with_host_bits_never_matches_like_cgminer() {
        let allow: ApiAllowList = "W:192.0.2.5/24".parse().unwrap();
        assert!(!allow.allows_write(ip("192.0.2.5")));
        assert!(!allow.allows_write(ip("192.0.2.0")));
    }

    #[test]
    fn grant_write_goes_before_the_catch_all() {
        let mut allow: ApiAllowList = "A:0/0,W:*".parse().unwrap();
        allow.grant_write(ip("192.0.2.10"));
        assert_eq!(allow.to_string(), "W:192.0.2.10,A:0/0,W:*");
        assert!(allow.allows_write(ip("192.0.2.10")));
        assert!(!allow.allows_write(ip("192.0.2.11")));

        // Granting again does not duplicate, and replaces a read rule for it.
        let mut allow: ApiAllowList = "R:192.0.2.10,A:0/0".parse().unwrap();
        allow.grant_write(ip("192.0.2.10"));
        allow.grant_write(ip("192.0.2.10"));
        assert_eq!(allow.to_string(), "W:192.0.2.10,A:0/0");

        // Subnets are truncated so cgminer can match them.
        let mut allow = ApiAllowList::default();
        let subnet = ApiAccessNetwork::new(ip("192.0.2.10"), 24).unwrap();
        allow.grant_write(subnet);
        assert_eq!(allow.to_string(), "W:192.0.2.0/24");
        assert!(allow.allows_write(ip("192.0.2.42")));

        allow.remove(subnet);
        assert!(allow.is_empty());
    }

    #[test]
    fn network_constructor_validates_prefix() {
        assert!(ApiAccessNetwork::new(ip("192.0.2.0"), 0).is_err());
        assert!(ApiAccessNetwork::new(ip("192.0.2.0"), 33).is_err());
        assert!(ApiAccessNetwork::new(ip("2001:db8::"), 128).is_ok());
        assert!(ApiAccessNetwork::host(ip("192.0.2.1")).is_host());
        assert_eq!(
            "192.0.2/25"
                .parse::<ApiAccessNetwork>()
                .unwrap()
                .to_string(),
            "192.0.2.0/25"
        );
        assert_eq!(
            "2001:db8::1"
                .parse::<ApiAccessNetwork>()
                .unwrap()
                .to_string(),
            "[2001:db8::1]"
        );
    }

    #[test]
    fn api_groups_round_trip() {
        let groups =
            ApiCommandGroup::parse_list("A:stats:pools:devs:summary:version,q:quit:restart:*")
                .unwrap();
        assert_eq!(groups[0].letter, 'A');
        assert_eq!(
            groups[0].commands,
            ["stats", "pools", "devs", "summary", "version"]
        );
        assert_eq!(groups[1].letter, 'Q');
        assert_eq!(
            ApiCommandGroup::format_list(&groups),
            "A:stats:pools:devs:summary:version,Q:quit:restart:*"
        );
        assert!(ApiCommandGroup::parse("AB:stats").is_err());
    }

    #[test]
    fn config_serializes_lists_as_cgminer_strings() -> anyhow::Result<()> {
        let config = ApiAccessConfig {
            listen: Some(true),
            network: Some(true),
            groups: Some(ApiCommandGroup::parse_list(
                "A:stats:pools:devs:summary:version",
            )?),
            allow: Some("W:192.0.2.10,A:0/0,W:*".parse().unwrap()),
        };
        let json = serde_json::to_value(&config)?;
        assert_eq!(
            json,
            serde_json::json!({
                "listen": true,
                "network": true,
                "groups": "A:stats:pools:devs:summary:version",
                "allow": "W:192.0.2.10,A:0/0,W:*",
            })
        );
        let back: ApiAccessConfig = serde_json::from_value(json)?;
        assert_eq!(back, config);
        assert!(back.allows_write(ip("192.0.2.10")));

        let disabled = ApiAccessConfig {
            listen: Some(false),
            ..config
        };
        assert!(!disabled.allows_write(ip("192.0.2.10")));

        let empty: ApiAccessConfig = serde_json::from_str("{}")?;
        assert_eq!(empty, ApiAccessConfig::default());
        Ok(())
    }
}
