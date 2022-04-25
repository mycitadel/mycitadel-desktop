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

use crate::model::{Wallet, WalletSettings};

#[derive(Getters)]
pub struct ViewModel {
    wallet: Wallet,
}

impl ViewModel {
    pub fn with(wallet: Wallet) -> ViewModel {
        ViewModel { wallet }
    }

    pub fn as_wallet(&self) -> &Wallet {
        &self.wallet
    }

    pub fn as_descriptor(&self) -> &WalletSettings {
        self.wallet.as_settings()
    }

    pub fn to_descriptor(&self) -> WalletSettings {
        self.wallet.to_settings()
    }
}
