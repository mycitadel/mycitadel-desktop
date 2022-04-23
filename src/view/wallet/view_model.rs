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

use std::path::{Path, PathBuf};

use crate::model::{FileDocument, Wallet, WalletDescriptor};

#[derive(Getters)]
pub struct ViewModel {
    wallet: Wallet,
    path: PathBuf,
}

impl ViewModel {
    pub fn with(wallet: Wallet, path: PathBuf) -> ViewModel {
        ViewModel {
            wallet,
            // TODO: Add suffix with wallet id
            path,
        }
    }

    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<usize, strict_encoding::Error> {
        self.path = path.as_ref().to_owned();
        self.wallet.write_file(path)
    }

    pub fn as_descriptor(&self) -> &WalletDescriptor {
        self.wallet.as_descriptor()
    }

    pub fn to_descriptor(&self) -> WalletDescriptor {
        self.wallet.to_descriptor()
    }

    pub fn set_descriptor(&mut self, descr: WalletDescriptor) {
        self.wallet.set_descriptor(descr)
    }
}
