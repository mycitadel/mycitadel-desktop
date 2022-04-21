// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use std::cmp::Ordering;
use std::collections::BTreeMap;
use std::fmt::{self, Display, Formatter};
use std::hash::{Hash, Hasher};

use bitcoin::secp256k1::PublicKey;
use bitcoin::util::bip32::{ChainCode, ChildNumber, DerivationPath, ExtendedPubKey, Fingerprint};
use bitcoin::Network;
use chrono::{DateTime, Utc};
use hwi::error::Error as HwiError;
use hwi::HWIDevice;
use wallet::hd::schemata::DerivationBlockchain;
use wallet::hd::{DerivationScheme, HardenedIndex, SegmentIndexes};

// TODO: Move to descriptor wallet or BPro
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
pub enum PublicNetwork {
    #[display("mainnet")]
    Mainnet,
    #[display("testnet")]
    Testnet,
    #[display("signet")]
    Signet,
}

impl From<PublicNetwork> for Network {
    fn from(network: PublicNetwork) -> Self {
        match network {
            PublicNetwork::Mainnet => Network::Bitcoin,
            PublicNetwork::Testnet => Network::Testnet,
            PublicNetwork::Signet => Network::Signet,
        }
    }
}

impl From<PublicNetwork> for DerivationBlockchain {
    fn from(network: PublicNetwork) -> Self {
        match network {
            PublicNetwork::Mainnet => DerivationBlockchain::Bitcoin,
            PublicNetwork::Testnet => DerivationBlockchain::Testnet,
            PublicNetwork::Signet => DerivationBlockchain::Testnet,
        }
    }
}

impl Default for PublicNetwork {
    fn default() -> Self {
        PublicNetwork::Testnet
    }
}

impl PublicNetwork {
    pub fn is_testnet(self) -> bool {
        matches!(self, PublicNetwork::Testnet | PublicNetwork::Signet)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Ownership {
    Mine,
    External,
}

#[derive(Clone)]
pub struct HardwareDevice {
    pub device: HWIDevice,
    pub device_type: String,
    pub model: String,
    pub default_account: HardenedIndex,
    pub default_xpub: ExtendedPubKey,
}

#[derive(Debug, Display, Error)]
#[display(doc_comments)]
pub enum Error {
    /// No devices detected or some of devices are locked
    NoDevices(HwiError),

    /// Device {1} ({2}, master fingerprint {0}) does not support used derivation schema {3} on
    /// {4}.
    DerivationNotSupported(
        Fingerprint,
        String,
        String,
        DerivationScheme,
        PublicNetwork,
        HwiError,
    ),
}

impl Error {
    pub fn into_hwi_error(self) -> HwiError {
        match self {
            Error::NoDevices(err) => err,
            Error::DerivationNotSupported(_, _, _, _, _, err) => err,
        }
    }
}

#[derive(Wrapper, Clone, Default, From)]
pub struct HardwareList(BTreeMap<Fingerprint, HardwareDevice>);

impl<'a> IntoIterator for &'a HardwareList {
    type Item = (&'a Fingerprint, &'a HardwareDevice);
    type IntoIter = std::collections::btree_map::Iter<'a, Fingerprint, HardwareDevice>;

    fn into_iter(self) -> Self::IntoIter {
        self.0.iter()
    }
}

impl HardwareList {
    pub fn enumerate(
        scheme: &DerivationScheme,
        network: PublicNetwork,
        default_account: HardenedIndex,
    ) -> Result<(HardwareList, Vec<Error>), Error> {
        let mut devices = bmap![];
        let mut log = vec![];

        for device in HWIDevice::enumerate().map_err(Error::NoDevices)? {
            let fingerprint = Fingerprint::from(&device.fingerprint[..]);

            let derivation = scheme.to_account_derivation(default_account.into(), network.into());
            let derivation_string = derivation.to_string();
            match device.get_xpub(
                &derivation_string.parse().expect(
                    "ancient bitcoin version with different derivation path implementation",
                ),
                network.is_testnet(),
            ) {
                Ok(hwikey) => {
                    let xpub = ExtendedPubKey {
                        network: network.into(),
                        depth: hwikey.xpub.depth,
                        parent_fingerprint: Fingerprint::from(&hwikey.xpub.parent_fingerprint[..]),
                        child_number: u32::from(hwikey.xpub.child_number).into(),
                        public_key: PublicKey::from_slice(&hwikey.xpub.public_key.key.serialize())
                            .expect("secp lib used by hwi is broken"),
                        chain_code: ChainCode::from(&hwikey.xpub.chain_code[..]),
                    };
                    devices.insert(
                        fingerprint,
                        HardwareDevice {
                            device_type: device.device_type.clone(),
                            model: device.model.clone(),
                            device,
                            default_account,
                            default_xpub: xpub,
                        },
                    );
                }
                Err(err) => {
                    log.push(Error::DerivationNotSupported(
                        fingerprint,
                        device.device_type,
                        device.model,
                        scheme.clone(),
                        network,
                        err,
                    ));
                }
            };
        }
        Ok((devices.into(), log))
    }
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum OriginFormat {
    Master,
    SubMaster(ChildNumber),
    Standard(DerivationScheme, HardenedIndex, PublicNetwork),
    Custom(DerivationPath),
}

impl Display for OriginFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            OriginFormat::Master => f.write_str("m/"),
            OriginFormat::SubMaster(account) => Display::fmt(account, f),
            OriginFormat::Standard(scheme, account, network) => Display::fmt(
                &scheme.to_account_derivation((*account).into(), (*network).into()),
                f,
            ),
            OriginFormat::Custom(path) => Display::fmt(path, f),
        }
    }
}

impl OriginFormat {
    pub fn with(path: &DerivationPath) -> OriginFormat {
        match DerivationScheme::from_derivation(&path) {
            scheme @ DerivationScheme::Bip44
            | scheme @ DerivationScheme::Bip84
            | scheme @ DerivationScheme::Bip49
            | scheme @ DerivationScheme::Bip86
            | scheme @ DerivationScheme::Bip45
            | scheme @ DerivationScheme::Bip87
            | scheme @ DerivationScheme::Bip48 { .. } => {
                let account = path[2].try_into().expect("DerivationScheme parser broken");
                let testnet = path[1].first_index() != 0;
                let network = if testnet {
                    PublicNetwork::Testnet
                } else {
                    PublicNetwork::Mainnet
                };
                OriginFormat::Standard(scheme, account, network)
            }
            DerivationScheme::Custom { .. } if path.is_empty() => OriginFormat::Master,
            DerivationScheme::Custom { .. } if path.len() == 1 => OriginFormat::SubMaster(path[0]),
            DerivationScheme::LnpBp43 { .. }
            | DerivationScheme::Bip43 { .. }
            | DerivationScheme::Custom { .. } => OriginFormat::Custom(path.clone()),
            _ => OriginFormat::Custom(path.clone()),
        }
    }

    pub fn account(&self) -> Option<HardenedIndex> {
        match self {
            OriginFormat::Master => None,
            OriginFormat::SubMaster(index) => (*index).try_into().ok(),
            OriginFormat::Standard(_, index, _) => Some(*index),
            OriginFormat::Custom(_) => None,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Signer {
    // TODO: Transform first 4 fields into a descriptor wallet AccountXpub type
    pub fingerprint: Fingerprint,
    pub origin: DerivationPath,
    pub account: Option<HardenedIndex>,
    pub xpub: ExtendedPubKey,
    pub device: Option<String>,
    pub name: String,
    pub ownership: Ownership,
}

impl PartialEq for Signer {
    // Two signers considered equal when their xpubs are equal
    fn eq(&self, other: &Self) -> bool {
        self.xpub == other.xpub
    }
}

impl Eq for Signer {}

impl Hash for Signer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.xpub.hash(state)
    }
}

impl PartialOrd for Signer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Signer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.xpub.cmp(&other.xpub)
    }
}

impl Signer {
    pub fn with_device(
        fingerprint: Fingerprint,
        device: HardwareDevice,
        schema: &DerivationScheme,
        network: PublicNetwork,
    ) -> Signer {
        Signer {
            fingerprint,
            device: Some(device.device_type),
            name: device.model.clone(),
            origin: schema.to_account_derivation(device.default_account.into(), network.into()),
            xpub: device.default_xpub,
            account: Some(device.default_account),
            ownership: Ownership::Mine,
        }
    }

    pub fn with_xpub(
        xpub: ExtendedPubKey,
        schema: &DerivationScheme,
        network: PublicNetwork,
    ) -> Self {
        let (fingerprint, origin) = match xpub.depth {
            0 => (xpub.fingerprint(), DerivationPath::default()),
            1 => (xpub.parent_fingerprint, vec![xpub.child_number].into()),
            _ => (
                Fingerprint::default(),
                schema
                    .to_account_derivation(xpub.child_number, network.into())
                    .into(),
            ),
        };
        let format = OriginFormat::with(&origin);
        Signer {
            fingerprint,
            device: None,
            name: "".to_string(),
            origin,
            xpub,
            account: format.account(),
            ownership: Ownership::External,
        }
    }

    pub fn account_string(&self) -> String {
        self.account
            .as_ref()
            .map(HardenedIndex::to_string)
            .unwrap_or_else(|| s!("n/a"))
    }

    pub fn origin_format(&self) -> OriginFormat {
        OriginFormat::with(&self.origin)
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum DescriptorClass {
    PreSegwit,
    SegwitV0,
    NestedV0,
    TaprootC0,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
pub enum SigsReq {
    #[display("all signatures")]
    All,
    #[display("at least {0} signatures")]
    AtLeast(u16),
    // TODO: Transform to vector
    #[display("signature by {0}")]
    Specific(Fingerprint),
    #[display("any signature")]
    Any,
}

impl Default for SigsReq {
    fn default() -> Self {
        SigsReq::All
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
pub enum TimelockReq {
    #[display("anytime")]
    Anytime,
    #[display("after {0}")]
    OlderTime(DateTime<Utc>),
    #[display("after {0} blocks")]
    OlderBlock(u32),
    #[display("after date {0}")]
    AfterTime(DateTime<Utc>),
    #[display("after block {0}")]
    AfterBlock(u32),
}

impl Default for TimelockReq {
    fn default() -> Self {
        TimelockReq::Anytime
    }
}

#[derive(
    Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, Display
)]
#[display("{sigs} {timelock}")]
pub struct SpendingCondition {
    pub sigs: SigsReq,
    pub timelock: TimelockReq,
}
