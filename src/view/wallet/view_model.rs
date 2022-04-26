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
use std::path::PathBuf;

use crate::model::{
    file, DescriptorClass, DescriptorError, ElectrumServer, FileDocument, Signer, Wallet,
    WalletSettings,
};

#[derive(Debug, Display, Error, From)]
#[display(inner)]
pub enum ModelError {
    #[from]
    Descriptor(DescriptorError),

    #[from]
    FileSave(file::Error),
}

#[derive(Getters)]
pub struct ViewModel {
    wallet: Wallet,
    path: PathBuf,
}

impl ViewModel {
    pub fn with(wallet: Wallet, path: PathBuf) -> ViewModel {
        ViewModel { wallet, path }
    }

    pub fn save(&mut self) -> Result<usize, file::Error> {
        self.wallet.write_file(&self.path)
    }

    pub fn as_wallet(&self) -> &Wallet {
        &self.wallet
    }
    pub fn as_wallet_mut(&mut self) -> &mut Wallet {
        &mut self.wallet
    }
    pub fn to_wallet(&self) -> Wallet {
        self.wallet.clone()
    }

    pub fn as_settings(&self) -> &WalletSettings {
        self.wallet.as_settings()
    }
    pub fn to_settings(&self) -> WalletSettings {
        self.wallet.to_settings()
    }

    pub fn update_descriptor(
        &mut self,
        signers: Vec<Signer>,
        descriptor_classes: BTreeSet<DescriptorClass>,
        electrum: ElectrumServer,
    ) -> Result<Option<&ElectrumServer>, ModelError> {
        self.wallet.update_signers(signers)?;
        for class in descriptor_classes {
            self.wallet.add_descriptor_class(class);
        }
        let electrum_updated = self.wallet.update_electrum(electrum);
        self.save()?;
        Ok(if electrum_updated {
            Some(self.wallet.as_settings().electrum())
        } else {
            None
        })
    }
}
