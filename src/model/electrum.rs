// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use crate::model::PublicNetwork;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub enum ElectrumSec {
    #[display("tor")]
    Tor,
    #[display("ssl")]
    Tls,
    #[display("tcp")]
    None,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[display("{sec}://{server}:{port}")]
pub struct ElectrumServer {
    pub sec: ElectrumSec,
    pub server: String,
    pub port: u16,
}

impl ElectrumServer {
    pub fn tls(preset: ElectrumPreset, network: PublicNetwork) -> ElectrumServer {
        ElectrumServer {
            sec: ElectrumSec::Tls,
            server: preset.to_string(),
            port: preset.electrum_port(ElectrumSec::Tls, network),
        }
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub enum ElectrumPreset {
    #[display("pandora.network")]
    MyCitadel,

    #[display("blockstream.info")]
    Blockstream,

    #[display("")]
    Custom,
}

impl ElectrumPreset {
    pub fn all() -> &'static [ElectrumPreset] {
        &[
            ElectrumPreset::MyCitadel,
            ElectrumPreset::Blockstream,
            ElectrumPreset::Custom,
        ]
    }

    pub fn presets() -> &'static [ElectrumPreset] {
        &[ElectrumPreset::MyCitadel, ElectrumPreset::Blockstream]
    }

    pub fn electrum_port(self, sec: ElectrumSec, network: PublicNetwork) -> u16 {
        match (self, sec, network) {
            (ElectrumPreset::MyCitadel, _, network) => network.electrum_port(),
            (ElectrumPreset::Blockstream, ElectrumSec::None, PublicNetwork::Mainnet) => 110,
            (ElectrumPreset::Blockstream, ElectrumSec::None, PublicNetwork::Testnet) => 143,
            (ElectrumPreset::Blockstream, ElectrumSec::Tls, PublicNetwork::Mainnet) => 700,
            (ElectrumPreset::Blockstream, ElectrumSec::Tls, PublicNetwork::Testnet) => 993,
            (_, _, network) => network.electrum_port(),
        }
    }
}
