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

use std::collections::BTreeMap;
use std::path::PathBuf;

use bitcoin::util::bip32::{ChildNumber, Fingerprint};
use bitcoin::Transaction;
use miniscript::ToPublicKey;
use wallet::psbt::Psbt;

use super::sign_row::SigningModel;
use crate::model::psbt::McKeys;
use crate::model::PublicNetwork;
use crate::view::psbt::sign_row::Signing;

#[derive(Debug)]
pub enum ModelParam {
    Open(PathBuf, Psbt, PublicNetwork),
    Create(Psbt, PublicNetwork),
}

impl ModelParam {
    pub fn into_psbt(self) -> Psbt {
        match self {
            ModelParam::Open(_, psbt, _) | ModelParam::Create(psbt, _) => psbt,
        }
    }

    pub fn network(&self) -> PublicNetwork {
        match self {
            ModelParam::Open(_, _, network) | ModelParam::Create(_, network) => *network,
        }
    }

    pub fn path(&self) -> Option<PathBuf> {
        match self {
            ModelParam::Open(path, _, _) => Some(path.clone()),
            ModelParam::Create(_, _) => None,
        }
    }
}

#[derive(Getters, Default)]
pub struct ViewModel {
    psbt: Psbt,
    finalized_tx: Option<Transaction>,
    path: Option<PathBuf>,
    signing: SigningModel,
    #[getter(as_copy)]
    network: PublicNetwork,
    #[getter(prefix = "is_", as_copy)]
    modified: bool,
}

impl ViewModel {
    pub fn with(psbt: Psbt, path: Option<PathBuf>, network: PublicNetwork) -> ViewModel {
        let mut model = ViewModel {
            modified: path.is_none(),
            psbt,
            finalized_tx: None,
            path,
            signing: SigningModel::new(),
            network,
        };
        model.parse_psbt();
        model
    }

    pub fn parse_psbt(&mut self) {
        self.signing.clear();

        // Information on required signatures, indexed by terminal keys
        let mut signing_keys =
            BTreeMap::<bitcoin::PublicKey, (Fingerprint, Fingerprint, u32, u32)>::new();
        for input in &self.psbt.inputs {
            for (pk, (_, (master_fp, derivation))) in &input.tap_key_origins {
                if derivation.len() > 1 && derivation[1] == (ChildNumber::Hardened { index: 1 }) {
                    self.network = PublicNetwork::Testnet;
                }
                let key = pk.to_public_key();
                let (fp, _, present, required) =
                    signing_keys.entry(key).or_insert((zero!(), zero!(), 0, 0));
                *fp = *master_fp;
                *required += 1;
                *present += input.tap_key_sig.map(|_| 1u32).unwrap_or_default()
                    + input
                        .tap_script_sigs
                        .keys()
                        .filter(|(xpk, _)| xpk == pk)
                        .count() as u32;
            }
            for (pk, (master_fp, derivation)) in &input.bip32_derivation {
                if derivation.len() > 1 && derivation[1] == (ChildNumber::Hardened { index: 1 }) {
                    self.network = PublicNetwork::Testnet;
                }
                let key = bitcoin::PublicKey::new(*pk);
                let (fp, _, present, required) =
                    signing_keys.entry(key).or_insert((zero!(), zero!(), 0, 0));
                *fp = *master_fp;
                *required += 1;
                *present += input
                    .partial_sigs
                    .get(&bitcoin::PublicKey::new(*pk))
                    .map(|_| 1u32)
                    .unwrap_or_default();
            }
        }

        for (account_xpub, (master_fp, _)) in &self.psbt.xpub {
            for (fp, account_fp, ..) in signing_keys.values_mut() {
                if fp == master_fp {
                    *account_fp = account_xpub.fingerprint();
                } else if *fp == account_xpub.fingerprint() {
                    *account_fp = *fp;
                    *fp = *master_fp;
                }
            }
        }

        let signers = signing_keys.into_iter().fold(
            BTreeMap::<Fingerprint, (u32, u32)>::new(),
            |mut signers, (_, (master_fp, _, p, r))| {
                let (present, required) = signers.entry(master_fp).or_default();
                *present += p;
                *required += r;
                signers
            },
        );

        for (signer_no, (master_fp, (present, required))) in signers.into_iter().enumerate() {
            let name = self
                .psbt
                .signer_name(master_fp)
                .unwrap_or_else(|| format!("Signer #{}", signer_no + 1));
            let info = Signing::with(&name, master_fp, present, required);
            self.signing.append(&info);
        }
    }

    pub fn replace_psbt(&mut self, psbt: Psbt) {
        self.psbt = psbt;
        self.parse_psbt();
    }

    pub fn set_path(&mut self, path: PathBuf) { self.path = Some(path); }

    pub fn set_network(&mut self, network: PublicNetwork) { self.network = network; }

    pub fn clear_finalized_tx(&mut self) { self.finalized_tx = None; }

    pub fn set_finalized_tx(&mut self, tx: Transaction) { self.finalized_tx = Some(tx); }
}
