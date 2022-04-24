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

use crate::model::PublicNetwork;
use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{self, PublicKey, SECP256K1};
use bitcoin::util::bip32::ExtendedPubKey;
use wallet::hd::{TerminalStep, TrackingAccount, XpubRef};

pub trait Unsatisfiable {
    type Param;
    fn unsatisfiable(_: Self::Param) -> Self;
}

impl Unsatisfiable for PublicKey {
    type Param = ();

    fn unsatisfiable(_: Self::Param) -> Self {
        let mut unspendable_key = PublicKey::from_secret_key(&SECP256K1, &secp256k1::ONE_KEY);
        unspendable_key
            .add_exp_assign(
                &SECP256K1,
                &sha256::Hash::hash(&unspendable_key.serialize()),
            )
            .unwrap();
        unspendable_key
    }
}

impl Unsatisfiable for ExtendedPubKey {
    type Param = PublicNetwork;

    fn unsatisfiable(network: Self::Param) -> Self {
        let unspendable_key = PublicKey::unsatisfiable(());
        let mut buf = Vec::with_capacity(78);
        buf.extend(if network.is_testnet() {
            [0x04u8, 0x35, 0x87, 0xCF]
        } else {
            [0x04u8, 0x88, 0xB2, 0x1E]
        });
        buf.extend([0u8; 5]); // depth + fingerprint
        buf.extend([0u8; 4]); // child no
        buf.extend(&unspendable_key.serialize()[1..]);
        buf.extend(&unspendable_key.serialize());
        ExtendedPubKey::decode(&buf).expect("broken unspendable key construction")
    }
}

impl Unsatisfiable for TrackingAccount {
    type Param = (PublicNetwork, Vec<TerminalStep>);

    fn unsatisfiable(param: Self::Param) -> Self {
        let (network, terminal_path) = param;
        TrackingAccount {
            master: XpubRef::Unknown,
            account_path: vec![],
            account_xpub: ExtendedPubKey::unsatisfiable(network),
            revocation_seal: None,
            terminal_path,
        }
    }
}
