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

use std::collections::BTreeSet;

use chrono::prelude::*;

use super::{Bip43, PublicNetwork, Signer, SpendingCondition, WalletFormat};
use crate::model::{SigsReq, TimelockReq};

/// Wallet template is a way to define constrained version of a wallet descriptor, but unlike
/// [`super::WalletDescriptor`] not having restrains on the internal consistency between amount of
/// signatures already present and condition parameters.
#[derive(Clone, Eq, PartialEq, Hash, Debug, Default)]
pub struct WalletTemplate {
    pub format: WalletFormat,
    pub expected_signer_count: Option<u16>,
    pub signers: BTreeSet<Signer>,
    pub conditions: Vec<SpendingCondition>,
    pub network: PublicNetwork,
}

impl WalletTemplate {
    pub fn singlesig(taproot: bool, network: PublicNetwork) -> WalletTemplate {
        let format = if taproot {
            Bip43::singlesig_segwit0()
        } else {
            Bip43::singlelsig_taproot()
        };
        WalletTemplate {
            format: format.into(),
            expected_signer_count: Some(1),
            signers: none!(),
            conditions: vec![SpendingCondition::default()],
            network,
        }
    }

    /// # Panics
    ///
    /// If `expect_signer_count` argument is `Some(1)` or `Some(0)`.
    pub fn multisig(network: PublicNetwork, expected_signer_count: Option<u16>) -> WalletTemplate {
        let now = Utc::now();
        let conditions = match expected_signer_count {
            None => vec![SpendingCondition::default()],
            Some(0) | Some(1) => unreachable!("WalletTemplate::multisig must expect > 1 signature"),
            Some(2) => vec![
                SpendingCondition {
                    sigs: SigsReq::All,
                    timelock: TimelockReq::Anytime,
                },
                SpendingCondition {
                    sigs: SigsReq::Any,
                    timelock: TimelockReq::AfterTime(now.with_year(now.year() + 2).unwrap()),
                },
            ],
            Some(count) => vec![
                SpendingCondition {
                    sigs: SigsReq::All,
                    timelock: TimelockReq::Anytime,
                },
                SpendingCondition {
                    sigs: SigsReq::AtLeast(count - 1),
                    timelock: TimelockReq::AfterTime(now.with_year(now.year() + 2).unwrap()),
                },
                SpendingCondition {
                    sigs: SigsReq::AtLeast(count - 2),
                    timelock: TimelockReq::AfterTime(now.with_year(now.year() + 5).unwrap()),
                },
            ],
        };
        WalletTemplate {
            format: Bip43::multisig_descriptor().into(),
            expected_signer_count,
            signers: none!(),
            conditions,
            network,
        }
    }
}
