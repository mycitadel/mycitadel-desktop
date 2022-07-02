// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime SA, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use bitcoin::psbt::raw::ProprietaryKey;
use bitcoin::util::bip32::Fingerprint;
use wallet::psbt::Psbt;

pub const MC_PSBT_GLOBAL_SIGNER_NAME: u8 = 0;

pub trait McKeys {
    fn signer_name(&self, master_fp: Fingerprint) -> Option<String>;

    fn set_signer_name(&mut self, master_fp: Fingerprint, name: &str);
}

impl McKeys for Psbt {
    fn signer_name(&self, master_fp: Fingerprint) -> Option<String> {
        let signer_name_key = ProprietaryKey {
            prefix: b"MyCitadel".to_vec(),
            subtype: MC_PSBT_GLOBAL_SIGNER_NAME,
            key: master_fp[..].to_vec(),
        };
        self.proprietary
            .get(&signer_name_key)
            .cloned()
            .map(String::from_utf8)
            .transpose()
            .ok()
            .flatten()
    }

    fn set_signer_name(&mut self, master_fp: Fingerprint, name: &str) {
        let signer_name_key = ProprietaryKey {
            prefix: b"MyCitadel".to_vec(),
            subtype: MC_PSBT_GLOBAL_SIGNER_NAME,
            key: master_fp[..].to_vec(),
        };
        let entry = self.proprietary.entry(signer_name_key).or_default();
        *entry = name.as_bytes().to_vec();
    }
}
