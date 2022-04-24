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
use std::path::{Path, PathBuf};

use crate::model::{
    file, DescriptorClass, DescriptorError, FileDocument, Signer, Wallet, WalletDescriptor,
};

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

    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<usize, file::Error> {
        self.path = path.as_ref().to_owned();
        self.wallet.write_file(path)
    }

    pub fn as_descriptor(&self) -> &WalletDescriptor {
        self.wallet.as_descriptor()
    }

    pub fn to_descriptor(&self) -> WalletDescriptor {
        self.wallet.to_descriptor()
    }

    pub fn update_descriptor(
        &mut self,
        signers: Vec<Signer>,
        descriptor_classes: BTreeSet<DescriptorClass>,
    ) -> Result<(), DescriptorError> {
        self.wallet.update_signers(signers)?;
        for class in descriptor_classes {
            self.wallet.add_descriptor_class(class);
        }
        Ok(())
    }
}
