use std::collections::BTreeMap;

use bitcoin::secp256k1::PublicKey;
use bitcoin::util::bip32::{ChainCode, ExtendedPubKey, Fingerprint};
use bitcoin::Network;
use chrono::{DateTime, Utc};
use hwi::error::Error as HwiError;
use hwi::HWIDevice;
use wallet::hd::schemata::DerivationBlockchain;
use wallet::hd::{DerivationScheme, HardenedIndex};

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

    fn into_iter(self) -> Self::IntoIter { self.0.iter() }
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
                    devices.insert(fingerprint, HardwareDevice {
                        device_type: device.device_type.clone(),
                        model: device.model.clone(),
                        device,
                        default_account,
                        default_xpub: xpub,
                    });
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

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Signer {
    pub fingerprint: Fingerprint,
    pub device: Option<String>,
    pub name: String,
    pub xpub: ExtendedPubKey,
    pub account: HardenedIndex,
    pub ownership: Ownership,
}

impl Signer {
    pub fn with(fingerprint: Fingerprint, device: HardwareDevice) -> Signer {
        Signer {
            fingerprint,
            device: Some(device.device_type),
            name: device.model.clone(),
            xpub: device.default_xpub,
            account: device.default_account,
            ownership: Ownership::Mine,
        }
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum DescriptorClass {
    PreSegwit,
    SegwitV0,
    NestedV0,
    TaprootC0,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum SigsReq {
    All,
    AtLeast(u16),
    Specific(Fingerprint),
    Any,
}

impl Default for SigsReq {
    fn default() -> Self { SigsReq::All }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum TimelockReq {
    Anytime,
    OlderTime(DateTime<Utc>),
    OlderBlock(u32),
    AfterTime(DateTime<Utc>),
    AfterBlock(u32),
}

impl Default for TimelockReq {
    fn default() -> Self { TimelockReq::Anytime }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct SpendingCondition {
    pub sigs: SigsReq,
    pub timelock: TimelockReq,
}
