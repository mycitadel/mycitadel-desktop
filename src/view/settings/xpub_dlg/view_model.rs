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

use relm::Sender;
use wallet::slip132::KeyApplication;

use crate::model::{DerivationType, Notification, XpubDescriptor};
use crate::view::settings;

pub struct ViewModel {
    pub(super) testnet: bool,
    pub(super) slip_format: Option<KeyApplication>,
    pub(super) xpub: Option<XpubDescriptor<DerivationType>>,
    pub(super) standard: DerivationType,
    pub(super) notification: Option<Notification>,
    pub(super) sender: Sender<settings::Msg>,
}

impl ViewModel {
    pub fn with(standard: DerivationType, sender: Sender<settings::Msg>) -> ViewModel {
        ViewModel {
            testnet: false,
            slip_format: None,
            standard,
            xpub: None,
            notification: None,
            sender,
        }
    }
}
