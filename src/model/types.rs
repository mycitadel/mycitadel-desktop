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

use crate::model::XpubkeyCore;
use bitcoin::secp256k1::PublicKey;
use bitcoin::util::bip32::{ChainCode, ChildNumber, DerivationPath, ExtendedPubKey, Fingerprint};
use bitcoin::{Network, OutPoint};
use chrono::{DateTime, Utc};
use hwi::error::Error as HwiError;
use hwi::HWIDevice;
use miniscript::descriptor::DescriptorType;
use wallet::hd::standards::DerivationBlockchain;
use wallet::hd::{
    AccountStep, Bip43, DerivationStandard, DerivationSubpath, HardenedIndex, SegmentIndexes,
    TerminalStep, TrackingAccount, UnhardenedIndex, XpubRef,
};

// TODO: Move to descriptor wallet or BPro
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
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
        Network::from(&network)
    }
}

impl From<&PublicNetwork> for Network {
    fn from(network: &PublicNetwork) -> Self {
        match network {
            PublicNetwork::Mainnet => Network::Bitcoin,
            PublicNetwork::Testnet => Network::Testnet,
            PublicNetwork::Signet => Network::Signet,
        }
    }
}

impl TryFrom<Network> for PublicNetwork {
    type Error = ();
    fn try_from(network: Network) -> Result<Self, Self::Error> {
        Ok(match network {
            Network::Bitcoin => PublicNetwork::Mainnet,
            Network::Testnet => PublicNetwork::Testnet,
            Network::Signet => PublicNetwork::Signet,
            Network::Regtest => return Err(()),
        })
    }
}

impl From<PublicNetwork> for DerivationBlockchain {
    fn from(network: PublicNetwork) -> Self {
        DerivationBlockchain::from(&network)
    }
}

impl From<&PublicNetwork> for DerivationBlockchain {
    fn from(network: &PublicNetwork) -> Self {
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

    pub fn electrum_port(self) -> u16 {
        match self {
            PublicNetwork::Mainnet => 50001,
            PublicNetwork::Testnet => 60001,
            PublicNetwork::Signet => 60601,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
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
    DerivationNotSupported(Fingerprint, String, String, Bip43, PublicNetwork, HwiError),
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
        scheme: &Bip43,
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
    Standard(Bip43, Option<HardenedIndex>, PublicNetwork),
    CustomAccount(DerivationPath),
    Custom(DerivationPath),
}

impl Display for OriginFormat {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        match self {
            OriginFormat::Master => f.write_str("m/"),
            OriginFormat::SubMaster(account) => Display::fmt(account, f),
            OriginFormat::Standard(scheme, _, network) => {
                Display::fmt(&scheme.to_origin_derivation((*network).into()), f)
            }
            OriginFormat::CustomAccount(path) => Display::fmt(path, f),
            OriginFormat::Custom(path) => Display::fmt(path, f),
        }
    }
}

impl OriginFormat {
    pub fn with_account(path: &DerivationPath, network: PublicNetwork) -> OriginFormat {
        let bip43 = Bip43::deduce(&path);
        if let Some(bip43) = bip43 {
            let account = bip43
                .extract_account_index(path)
                .transpose()
                .expect("BIP43 parser is broken");
            OriginFormat::Standard(bip43, account, network)
        } else if path.is_empty() {
            OriginFormat::Master
        } else if path.len() == 1 {
            OriginFormat::SubMaster(path[0])
        } else {
            let path = path.as_ref().to_vec();
            let account = path.last().unwrap();
            if account.is_hardened() {
                OriginFormat::CustomAccount(path.into())
            } else {
                OriginFormat::Custom(path.into())
            }
        }
    }

    pub fn account(&self) -> Option<HardenedIndex> {
        match self {
            OriginFormat::Master => None,
            OriginFormat::SubMaster(index) => (*index).try_into().ok(),
            OriginFormat::Standard(_, index, _) => *index,
            OriginFormat::Custom(_) => None,
            OriginFormat::CustomAccount(_) => None,
        }
    }

    pub fn master_fingerprint_editable(&self) -> bool {
        match self {
            OriginFormat::Master => false,
            OriginFormat::SubMaster(_) => false,
            OriginFormat::Standard(s, _, network) => {
                s.to_origin_derivation((*network).into()).len() > 1
            }
            OriginFormat::Custom(derivation) | OriginFormat::CustomAccount(derivation) => {
                derivation.len() > 1
            }
        }
    }
}

#[derive(Clone, Debug)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub struct Signer {
    pub master_fp: Fingerprint,
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
        self.xpub_core() == other.xpub_core()
    }
}

impl Eq for Signer {}

impl Hash for Signer {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.xpub.identifier().hash(state)
    }
}

impl PartialOrd for Signer {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for Signer {
    fn cmp(&self, other: &Self) -> Ordering {
        self.xpub_core().cmp(&other.xpub_core())
    }
}

impl Signer {
    pub fn with_device(
        fingerprint: Fingerprint,
        device: HardwareDevice,
        schema: &Bip43,
        network: PublicNetwork,
    ) -> Signer {
        Signer {
            master_fp: fingerprint,
            device: Some(device.device_type),
            name: device.model.clone(),
            origin: schema.to_account_derivation(device.default_account.into(), network.into()),
            xpub: device.default_xpub,
            account: Some(device.default_account),
            ownership: Ownership::Mine,
        }
    }

    pub fn with_xpub(xpub: ExtendedPubKey, schema: &Bip43, network: PublicNetwork) -> Self {
        let (fingerprint, origin, account) = match (xpub.depth, schema.account_depth()) {
            (0, _) => (xpub.fingerprint(), empty!(), None),
            (1, _) => (
                xpub.parent_fingerprint,
                vec![xpub.child_number].into(),
                HardenedIndex::try_from(xpub.child_number).ok(),
            ),
            (depth, Some(account_depth))
                if xpub.child_number.is_hardened() && depth == account_depth =>
            {
                let coin_depth = schema.coin_type_depth().unwrap_or(account_depth);
                let max_depth = coin_depth.max(account_depth) as usize;
                let min_depth = coin_depth.min(account_depth) as usize;
                let path = if max_depth - min_depth != 1 {
                    vec![xpub.child_number]
                } else {
                    let mut path = vec![ChildNumber::zero(); 2];
                    path[coin_depth as usize - min_depth] =
                        DerivationBlockchain::from(network).coin_type().into();
                    path[account_depth as usize - min_depth] = xpub.child_number;
                    path
                };
                (
                    zero!(),
                    path.into(),
                    HardenedIndex::try_from(xpub.child_number).ok(),
                )
            }
            _ => (
                zero!(),
                vec![xpub.child_number].into(),
                HardenedIndex::try_from(xpub.child_number).ok(),
            ),
        };
        Signer {
            master_fp: fingerprint,
            device: None,
            name: "".to_string(),
            origin,
            xpub,
            account,
            ownership: Ownership::External,
        }
    }

    pub fn is_master_known(&self) -> bool {
        self.master_fp != zero!()
    }

    pub fn account_string(&self) -> String {
        self.account
            .as_ref()
            .map(HardenedIndex::to_string)
            .unwrap_or_else(|| s!("n/a"))
    }

    pub fn origin_format(&self, network: PublicNetwork) -> OriginFormat {
        OriginFormat::with_account(&self.origin, network)
    }

    pub fn xpub_core(&self) -> XpubkeyCore {
        XpubkeyCore::from(self.xpub)
    }

    pub fn fingerprint(&self) -> Fingerprint {
        self.xpub.fingerprint()
    }

    pub fn master_xpub(&self) -> XpubRef {
        if self.is_master_known() {
            XpubRef::Fingerprint(self.master_fp)
        } else {
            XpubRef::Unknown
        }
    }

    pub fn to_tracking_account(&self, terminal_path: Vec<TerminalStep>) -> TrackingAccount {
        let path: Vec<ChildNumber> = self.origin.clone().into();
        TrackingAccount {
            master: self.master_xpub(),
            account_path: path
                .into_iter()
                .map(AccountStep::try_from)
                .collect::<Result<_, _>>()
                .expect("inconsistency in constructed derivation path"),
            account_xpub: self.xpub,
            revocation_seal: None,
            terminal_path,
        }
    }
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub enum DescriptorClass {
    PreSegwit,
    SegwitV0,
    NestedV0,
    TaprootC0,
}

impl From<&DescriptorType> for DescriptorClass {
    fn from(ty: &DescriptorType) -> Self {
        match ty {
            DescriptorType::Bare
            | DescriptorType::Sh
            | DescriptorType::ShSortedMulti
            | DescriptorType::Pkh => DescriptorClass::PreSegwit,
            DescriptorType::Wpkh | DescriptorType::WshSortedMulti | DescriptorType::Wsh => {
                DescriptorClass::SegwitV0
            }
            DescriptorType::ShWsh | DescriptorType::ShWshSortedMulti | DescriptorType::ShWpkh => {
                DescriptorClass::NestedV0
            }
            DescriptorType::Tr => DescriptorClass::TaprootC0,
        }
    }
}

impl From<DescriptorType> for DescriptorClass {
    fn from(ty: DescriptorType) -> Self {
        DescriptorClass::from(&ty)
    }
}

impl DescriptorClass {
    pub fn bip43(self, sigs_no: usize) -> Bip43 {
        match (self, sigs_no > 1) {
            (DescriptorClass::PreSegwit, false) => Bip43::singlesig_pkh(),
            (DescriptorClass::SegwitV0, false) => Bip43::singlesig_segwit0(),
            (DescriptorClass::NestedV0, false) => Bip43::singlesig_nested0(),
            (DescriptorClass::TaprootC0, false) => Bip43::singlelsig_taproot(),
            (DescriptorClass::PreSegwit, true) => Bip43::multisig_ordered_sh(),
            (DescriptorClass::SegwitV0, true) => Bip43::multisig_segwit0(),
            (DescriptorClass::NestedV0, true) => Bip43::multisig_nested0(),
            (DescriptorClass::TaprootC0, true) => Bip43::multisig_descriptor(),
        }
    }

    pub fn is_segwit_v0(self) -> bool {
        match self {
            DescriptorClass::SegwitV0 | DescriptorClass::NestedV0 => true,
            DescriptorClass::PreSegwit | DescriptorClass::TaprootC0 => false,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
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
#[derive(StrictEncode, StrictDecode)]
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
pub enum TimelockReq {
    #[display("anytime")]
    Anytime,
    #[display("after {0}")]
    OlderTime(DateTime<Utc>),
    #[display("after {0} blocks")]
    OlderBlock(u16),
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
#[cfg_attr(
    feature = "serde",
    derive(Serialize, Deserialize),
    serde(crate = "serde_crate")
)]
#[derive(StrictEncode, StrictDecode)]
#[display("{sigs} {timelock}")]
pub struct TimelockedSigs {
    pub sigs: SigsReq,
    pub timelock: TimelockReq,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Prevout {
    pub outpoint: OutPoint,
    pub amount: u64,
    pub change: bool,
    pub index: UnhardenedIndex,
}

impl Prevout {
    pub fn terminal(&self) -> DerivationSubpath<UnhardenedIndex> {
        DerivationSubpath::from(
            &[
                if self.change {
                    UnhardenedIndex::one()
                } else {
                    UnhardenedIndex::zero()
                },
                self.index,
            ][..],
        )
    }
}
