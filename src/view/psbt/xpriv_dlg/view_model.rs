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

use bitcoin::util::bip32::{ExtendedPrivKey, Fingerprint};
use relm::Sender;
use wallet::psbt::Psbt;

use crate::model::Notification;
use crate::view::psbt;

pub struct ViewModel {
    pub(super) testnet: bool,
    pub(super) psbt: Psbt,
    pub(super) master_fp: Fingerprint,
    pub(super) xpriv: Option<ExtendedPrivKey>,
    pub(super) notification: Option<Notification>,
    pub(super) sender: Sender<psbt::Msg>,
}

impl ViewModel {
    pub fn with(
        testnet: bool,
        psbt: Psbt,
        master_fp: Fingerprint,
        sender: Sender<psbt::Msg>,
    ) -> ViewModel {
        ViewModel {
            testnet,
            psbt,
            master_fp,
            xpriv: None,
            notification: None,
            sender,
        }
    }
}
