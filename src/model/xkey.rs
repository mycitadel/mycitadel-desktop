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

use bitcoin::hashes::Hash;
use bitcoin::util::bip32;
use bitcoin::util::bip32::{ChainCode, ChildNumber, DerivationPath, ExtendedPubKey, Fingerprint};
use bitcoin::{secp256k1, XpubIdentifier};
use std::fmt::Display;
use std::io::Write;
use std::str::FromStr;
use wallet::hd::{DerivationStandard, HardenedIndex, UnhardenedIndex};
use wallet::slip132;
use wallet::slip132::{DefaultResolver, FromSlip132, KeyVersion};

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error)]
#[display(doc_comments)]
pub enum XpubRequirementError {
    /// the provided extended public key can't be used under the required
    /// derivation standard. The public key is suitable for {actual_standard}
    /// derivations, while a key for {required_standard} is needed.
    StandardMismatch {
        actual_standard: String,
        required_standard: String,
    },

    /// the provided extended public key has a derivation depth {actual_depth},
    /// whis is less than the depth of account-level key {required_depth}
    /// according to {standard}.
    ShallowKey {
        required_depth: u8,
        actual_depth: u8,
        standard: String,
    },

    /// extended public key is invalid for the provided requirements.
    /// Specifically, network information in BIP-32 data ({bip_network}) does
    /// not match network information encoded in SLIP-132 key version prefix
    /// ({slip_network}).
    NetworkMismatch {
        slip_network: bitcoin::Network,
        bip_network: bitcoin::Network,
    },

    /// the given key is an account key according to the provided standard {0},
    /// however it uses a non-hardened derivation index {1}.
    UnhardenedAccountKey(String, UnhardenedIndex),
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error)]
#[display(doc_comments)]
pub enum NonStandardDerivation {
    /// the given key is invalid or the derivation path is invalid due to
    /// account-level key being derived at non-hardened index {0}.
    UnhardenedAccount(UnhardenedIndex),

    /// non-standard derivation path with coin type being a non-hardened index {0}.
    UnhardenedCoinType(UnhardenedIndex),
}

/// Deterministic part of the extended public key descriptor
#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
#[derive(StrictEncode, StrictDecode)]
pub struct XpubkeyCore {
    pub public_key: secp256k1::PublicKey,
    pub chain_code: ChainCode,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct XpubOrigin<Standard>
where
    Standard: DerivationStandard,
{
    pub testnet: bool,
    pub master_fingerprint: Option<Fingerprint>,
    pub standard: Option<Standard>,
    pub account: Option<HardenedIndex>,
}

impl<Standard> XpubOrigin<Standard>
where
    Standard: DerivationStandard + ToString,
{
    /// Constructs origin information for _an account_-level xpub or deeper key,
    /// extracting it from both `xpub` and SLIP132 key version (prefix) data.
    /// Ensures consistency of this information and returns error indicating
    /// discovered inconsistency.
    ///
    /// Compares the following correspondences between xpub and SLIP132-encoded
    /// key version:
    /// - network (testnet/mainnet only, since SLIP132 does not cover more
    ///   networks for bitcoin);
    /// - specific BIP43-based derivation standard matching the possible use
    ///   of the extended public key as an account-level key or deeper;
    ///   basing on its depth and child number;
    /// - if the xpub depth matches account key depth defined by the provided
    ///   derivation standard information, the child number of the xpub must be
    ///   a hardened number.
    ///
    /// Also checks that if there is a provided SLIP132 key version and
    /// derivation standard, they do match.
    pub fn with(
        master_fingerprint: Option<Fingerprint>,
        xpub: ExtendedPubKey,
        standard: Option<Standard>,
        slip: Option<KeyVersion>,
    ) -> Result<XpubOrigin<Standard>, XpubRequirementError> {
        let application = slip
            .as_ref()
            .and_then(KeyVersion::application::<DefaultResolver>);
        let standard_slip = application.and_then(Standard::matching);

        match (&standard, &standard_slip) {
            (Some(bip43), Some(slip)) if bip43 != slip => {
                return Err(XpubRequirementError::StandardMismatch {
                    actual_standard: slip.to_string(),
                    required_standard: bip43.to_string(),
                });
            }
            _ => {}
        }

        match slip
            .as_ref()
            .and_then(KeyVersion::network::<DefaultResolver>)
        {
            Some(slip_network) if slip_network != xpub.network => {
                return Err(XpubRequirementError::NetworkMismatch {
                    slip_network,
                    bip_network: xpub.network,
                });
            }
            _ => {}
        }

        let account_depth = standard_slip
            .as_ref()
            .and_then(DerivationStandard::account_depth);
        let account = match (&standard_slip, account_depth) {
            (Some(standard_slip), Some(required_depth)) if xpub.depth < required_depth => {
                return Err(XpubRequirementError::ShallowKey {
                    required_depth,
                    actual_depth: xpub.depth,
                    standard: standard_slip.to_string(),
                });
            }
            (Some(standard_slip), _) => {
                Some(HardenedIndex::try_from(xpub.child_number).map_err(|err| {
                    XpubRequirementError::UnhardenedAccountKey(standard_slip.to_string(), err.0)
                })?)
            }
            _ => None,
        };

        Ok(XpubOrigin {
            testnet: xpub.network == bitcoin::Network::Bitcoin,
            master_fingerprint,
            standard: standard.or(standard_slip),
            account,
        })
    }

    /// Deduces key origin information, using derivation path, internal key
    /// metadata and optional SLIP132 version prefix.
    ///
    /// # Returns
    ///
    /// The function ensures that the derivation path matches the standard which
    /// is defined by SLIP132, if the slip information is provided, and errors
    /// with [`NonStandardDerivation`] otherwise. This "extenral" error returned
    /// by the function may indicate the internal inconsistency in the program
    /// logic and can be `expect`'ed in this case.
    ///
    /// The function also checks the key and SLIP132 data for the internal
    /// consistency using [`XpubOrigin::with`] method, and returns
    /// `Ok(`[`XpubRequirementError`]`)` if this check fails. It also checks
    /// that the provided derivation path coin type index matches the network
    /// specified by the SLIP132 and xpub data, also returning
    /// `Ok(`[`XpubRequirementError`]`)` if this check fails. These errors
    /// should not be ignored.
    pub fn deduce(
        master_fingerprint: Option<Fingerprint>,
        source: &DerivationPath,
        xpub: ExtendedPubKey,
        slip: Option<KeyVersion>,
    ) -> Result<Result<XpubOrigin<Standard>, XpubRequirementError>, NonStandardDerivation> {
        let standard = Standard::deduce(source);

        if let Some(ref standard) = standard {
            standard
                .extract_account_index(source)
                .transpose()
                .map_err(|err| NonStandardDerivation::UnhardenedAccount(err.0))?;

            if let Some(network) = slip
                .as_ref()
                .and_then(KeyVersion::network::<DefaultResolver>)
            {
                if let Some(standard_network) = standard
                    .network(source)
                    .transpose()
                    .map_err(|err| NonStandardDerivation::UnhardenedCoinType(err.0))?
                {
                    if standard_network != network {
                        return Ok(Err(XpubRequirementError::NetworkMismatch {
                            slip_network: network,
                            bip_network: standard_network,
                        }));
                    }
                }
            }
        }

        Ok(XpubOrigin::with(master_fingerprint, xpub, standard, slip))
    }
}

#[derive(Getters, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct XpubDescriptor<Standard>
where
    Standard: DerivationStandard,
{
    #[getter(as_copy)]
    testnet: bool,
    #[getter(as_copy)]
    depth: u8,
    #[getter(as_copy)]
    parent_fingerprint: Fingerprint,
    #[getter(as_copy)]
    child_number: ChildNumber,
    #[getter(as_copy)]
    public_key: secp256k1::PublicKey,
    #[getter(as_copy)]
    chain_code: ChainCode,

    #[getter(as_copy, as_mut)]
    master_fingerprint: Option<Fingerprint>,
    #[getter(as_ref)]
    standard: Option<Standard>,
    #[getter(as_copy, as_mut)]
    account: Option<HardenedIndex>,
}

#[derive(
    Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display, Error, From
)]
#[display(inner)]
pub enum XpubParseError {
    #[from]
    Bip32(bip32::Error),

    #[from]
    Slip132(slip132::Error),

    #[from]
    Inconsistency(XpubRequirementError),
}

impl<Standard> FromStr for XpubDescriptor<Standard>
where
    Standard: DerivationStandard + Display,
{
    type Err = XpubParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        // The string here could be just a xpub, slip132 xpub or xpub prefixed
        // with origin information in a different formats.

        // TODO: Implement `[fp/derivation/path]xpub` processing
        // TODO: Implement `m=[fp]/derivation/path/account=[xpub]` processing

        let xpub = ExtendedPubKey::from_str(s).or_else(|_| ExtendedPubKey::from_slip132_str(s))?;

        let slip = KeyVersion::from_xkey_str(s).ok();

        XpubDescriptor::with(None, xpub, None, slip).map_err(XpubParseError::from)
    }
}

impl<Standard> From<ExtendedPubKey> for XpubDescriptor<Standard>
where
    Standard: DerivationStandard,
{
    fn from(xpub: ExtendedPubKey) -> Self {
        XpubDescriptor {
            testnet: xpub.network != bitcoin::Network::Bitcoin,
            depth: xpub.depth,
            parent_fingerprint: xpub.parent_fingerprint,
            child_number: xpub.child_number,
            public_key: xpub.public_key,
            chain_code: xpub.chain_code,
            master_fingerprint: None,
            standard: None,
            account: None,
        }
    }
}

impl<Standard> From<&XpubDescriptor<Standard>> for ExtendedPubKey
where
    Standard: DerivationStandard,
{
    fn from(xpub: &XpubDescriptor<Standard>) -> Self {
        ExtendedPubKey {
            network: if xpub.testnet {
                bitcoin::Network::Testnet
            } else {
                bitcoin::Network::Bitcoin
            },
            depth: xpub.depth,
            parent_fingerprint: xpub.parent_fingerprint,
            child_number: xpub.child_number,
            public_key: xpub.public_key,
            chain_code: xpub.chain_code,
        }
    }
}

impl<Standard> From<XpubDescriptor<Standard>> for ExtendedPubKey
where
    Standard: DerivationStandard,
{
    fn from(xpub: XpubDescriptor<Standard>) -> Self {
        ExtendedPubKey::from(&xpub)
    }
}

impl<Standard> XpubDescriptor<Standard>
where
    Standard: DerivationStandard + ToString,
{
    /// Constructs origin information for _an account_-level xpub or deeper key,
    /// parsing it from a given xpub descriptor string.
    /// Ensures consistency of this information and returns error indicating
    /// discovered inconsistency.
    ///
    /// Compares the following correspondences between xpub and SLIP132-encoded
    /// key version:
    /// - network (testnet/mainnet only, since SLIP132 does not cover more
    ///   networks for bitcoin);
    /// - specific BIP43-based derivation standard matching the possible use
    ///   of the extended public key as an account-level key or deeper;
    ///   basing on its depth and child number;
    /// - if the xpub depth matches account key depth defined by the provided
    ///   derivation standard information, the child number of the xpub must be
    ///   a hardened number.
    ///
    /// Also checks that if there is a provided SLIP132 key version and
    /// derivation standard, they do match.
    pub fn from_str_checked(
        s: &str,
        standard: Option<Standard>,
    ) -> Result<XpubDescriptor<Standard>, XpubParseError>
    where
        Standard: Display,
    {
        let mut xd = XpubDescriptor::from_str(s)?;

        match (&standard, &xd.standard) {
            (Some(required), Some(actual)) if required != actual => {}
            _ => {}
        }
        xd.standard = standard.or(xd.standard);

        Ok(xd)
    }

    /// Constructs origin information for _an account_-level xpub or deeper key,
    /// extracting it from both `xpub` and SLIP132 key version (prefix) data.
    /// Ensures consistency of this information and returns error indicating
    /// discovered inconsistency.
    ///
    /// Compares the following correspondences between xpub and SLIP132-encoded
    /// key version:
    /// - network (testnet/mainnet only, since SLIP132 does not cover more
    ///   networks for bitcoin);
    /// - specific BIP43-based derivation standard matching the possible use
    ///   of the extended public key as an account-level key or deeper;
    ///   basing on its depth and child number;
    /// - if the xpub depth matches account key depth defined by the provided
    ///   derivation standard information, the child number of the xpub must be
    ///   a hardened number.
    ///
    /// Also checks that if there is a provided SLIP132 key version and
    /// derivation standard, they do match.
    pub fn with(
        master_fingerprint: Option<Fingerprint>,
        xpub: ExtendedPubKey,
        standard: Option<Standard>,
        slip: Option<KeyVersion>,
    ) -> Result<XpubDescriptor<Standard>, XpubRequirementError> {
        let mut xd = XpubDescriptor::from(xpub);
        let origin = XpubOrigin::with(master_fingerprint, xpub, standard, slip)?;
        xd.standard = origin.standard;
        xd.master_fingerprint = master_fingerprint;
        xd.account = origin.account;
        Ok(xd)
    }

    /// Deduces key origin information, using derivation path, internal key
    /// metadata and optional SLIP132 version prefix.
    ///
    /// # Returns
    ///
    /// The function ensures that the derivation path matches the standard which
    /// is defined by SLIP132, if the slip information is provided, and errors
    /// with [`NonStandardDerivation`] otherwise. This "extenral" error returned
    /// by the function may indicate the internal inconsistency in the program
    /// logic and can be `expect`'ed in this case.
    ///
    /// The function also checks the key and SLIP132 data for the internal
    /// consistency using [`XpubOrigin::with`] method, and returns
    /// `Ok(`[`XpubRequirementError`]`)` if this check fails. It also checks
    /// that the provided derivation path coin type index matches the network
    /// specified by the SLIP132 and xpub data, also returning
    /// `Ok(`[`XpubRequirementError`]`)` if this check fails. These errors
    /// should not be ignored.
    pub fn deduce(
        master_fingerprint: Option<Fingerprint>,
        source: &DerivationPath,
        xpub: ExtendedPubKey,
        slip: Option<KeyVersion>,
    ) -> Result<Result<XpubDescriptor<Standard>, XpubRequirementError>, NonStandardDerivation> {
        let mut xd = XpubDescriptor::from(xpub);
        let origin = match XpubOrigin::deduce(master_fingerprint, source, xpub, slip) {
            Err(err) => return Err(err),
            Ok(Err(err)) => return Ok(Err(err)),
            Ok(Ok(origin)) => origin,
        };
        xd.standard = origin.standard;
        xd.master_fingerprint = master_fingerprint;
        xd.account = origin.account;
        Ok(Ok(xd))
    }
}

impl<Standard> XpubDescriptor<Standard>
where
    Standard: DerivationStandard,
{
    pub fn identifier(&self) -> XpubIdentifier {
        let mut engine = XpubIdentifier::engine();
        engine
            .write_all(&self.public_key.serialize())
            .expect("engines don't error");
        XpubIdentifier::from_engine(engine)
    }

    pub fn fingerprint(&self) -> Fingerprint {
        Fingerprint::from(&self.identifier()[0..4])
    }

    pub fn to_origin(&self) -> XpubOrigin<Standard> {
        XpubOrigin {
            testnet: self.testnet,
            master_fingerprint: self.master_fingerprint,
            standard: self.standard.clone(),
            account: self.account,
        }
    }

    pub fn into_origin(self) -> XpubOrigin<Standard> {
        XpubOrigin {
            testnet: self.testnet,
            master_fingerprint: self.master_fingerprint,
            standard: self.standard,
            account: self.account,
        }
    }
}
