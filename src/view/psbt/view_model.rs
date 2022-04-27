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
use std::{fs, io};

use bitcoin::consensus::Encodable;
use bitcoin::hashes::Hash;
use bitcoin::psbt::raw::ProprietaryKey;
use bitcoin::psbt::serialize::Serialize;
use bitcoin::psbt::PartiallySignedTransaction;
use bitcoin::util::bip32::Fingerprint;
use bitcoin::{secp256k1, XOnlyPublicKey, XpubIdentifier};
use miniscript::ToPublicKey;
use wallet::psbt::Psbt;

use super::sign_row::SigningModel;
use crate::view::psbt::sign_row::Signing;

pub const MC_PSBT_GLOBAL_SIGNER_NAME: u8 = 0;

#[derive(Getters, Default)]
pub struct ViewModel {
    psbt: Psbt,
    path: PathBuf,
    pub signing: SigningModel,
}

impl ViewModel {
    pub fn with(psbt: Psbt, path: PathBuf) -> ViewModel {
        let mut keys = BTreeMap::<Fingerprint, (Fingerprint, u32, u32)>::new();
        let mut bpk = BTreeMap::<secp256k1::PublicKey, Fingerprint>::new();
        let mut xpk = BTreeMap::<XOnlyPublicKey, Fingerprint>::new();
        for input in &psbt.inputs {
            for (pk, (_, (fingerprint, _))) in &input.tap_key_origins {
                let (fp, present, required) = keys.entry(*fingerprint).or_insert((zero!(), 0, 0));
                *fp = Fingerprint::from(
                    &XpubIdentifier::hash(&pk.to_public_key().serialize()[..])[0..4],
                );
                *required += 1;
                *present += input.tap_key_sig.map(|_| 1u32).unwrap_or_default()
                    + input
                        .tap_script_sigs
                        .keys()
                        .filter(|(xpk, _)| xpk == pk)
                        .count() as u32;
                xpk.entry(*pk).or_insert(*fingerprint);
            }
            for (pk, (fingerprint, _)) in &input.bip32_derivation {
                let (fp, present, required) = keys.entry(*fingerprint).or_insert((zero!(), 0, 0));
                *fp = Fingerprint::from(&XpubIdentifier::hash(&pk.serialize()[..])[0..4]);
                *required += 1;
                *present += input
                    .partial_sigs
                    .get(&bitcoin::PublicKey::new(*pk))
                    .map(|_| 1u32)
                    .unwrap_or_default();
                bpk.entry(*pk).or_insert(*fingerprint);
            }
        }

        let signer_name_key = ProprietaryKey {
            prefix: b"MyCitadel".to_vec(),
            subtype: MC_PSBT_GLOBAL_SIGNER_NAME,
            key: vec![],
        };
        let signing = SigningModel::new();
        for (fingerprint, (fp, present, required)) in keys {
            let name = psbt
                .proprietary
                .get(&signer_name_key)
                .cloned()
                .map(String::from_utf8)
                .transpose()
                .ok()
                .flatten()
                .unwrap_or(format!("{}", fp));
            let info = Signing::with(&name, fingerprint, present, required);
            signing.append(&info);
        }

        ViewModel {
            psbt,
            path,
            signing,
        }
    }

    pub fn save(&mut self) -> Result<usize, io::Error> {
        let psbt = PartiallySignedTransaction::from(self.psbt.clone());
        let file = fs::File::create(&self.path)?;
        psbt.consensus_encode(file)
    }
}
