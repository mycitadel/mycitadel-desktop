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

use bitcoin::secp256k1::{PublicKey, Secp256k1, SecretKey, SECP256K1};
use bitcoin::util::bip32::{DerivationPath, ExtendedPrivKey, Fingerprint};
use bitcoin::{secp256k1, KeyPair, XOnlyPublicKey};
use miniscript::ToPublicKey;
use wallet::psbt::sign::{SecretProvider, SecretProviderError};

#[derive(Debug)]
pub struct XprivSigner {
    pub xpriv: ExtendedPrivKey,
    pub master_fp: Fingerprint,
    pub secp: Secp256k1<secp256k1::All>,
}

impl XprivSigner {
    pub fn derive_xpriv(
        &self,
        fingerprint: Fingerprint,
        derivation: &DerivationPath,
        pubkey: PublicKey,
    ) -> Result<ExtendedPrivKey, SecretProviderError> {
        let derivation = if self.xpriv.fingerprint(SECP256K1) == fingerprint {
            derivation.clone()
        } else if self.master_fp == fingerprint {
            let remaining_derivation = derivation
                .into_iter()
                .skip_while(|child| child.is_hardened());
            let remaining_derivation = remaining_derivation.copied().collect();
            remaining_derivation
        } else {
            return Err(SecretProviderError::AccountUnknown(fingerprint, pubkey));
        };

        let sk = self
            .xpriv
            .derive_priv(SECP256K1, &derivation)
            .expect("xpriv derivation does not fail");

        Ok(sk)
    }
}

impl SecretProvider<secp256k1::All> for XprivSigner {
    fn secp_context(&self) -> &Secp256k1<secp256k1::All> { &self.secp }

    fn secret_key(
        &self,
        fingerprint: Fingerprint,
        derivation: &DerivationPath,
        pubkey: PublicKey,
    ) -> Result<SecretKey, SecretProviderError> {
        let xpriv = self.derive_xpriv(fingerprint, derivation, pubkey)?;
        Ok(xpriv.private_key)
    }

    fn key_pair(
        &self,
        fingerprint: Fingerprint,
        derivation: &DerivationPath,
        pubkey: XOnlyPublicKey,
    ) -> Result<KeyPair, SecretProviderError> {
        let xpriv = self.derive_xpriv(fingerprint, derivation, pubkey.to_public_key().inner)?;
        let sk = KeyPair::from_secret_key(SECP256K1, xpriv.private_key);
        Ok(sk)
    }

    fn use_musig(&self) -> bool { false }
}
