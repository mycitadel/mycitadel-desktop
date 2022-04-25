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

use gtk::prelude::*;
use gtk::{ListStore, TreeIter};
use wallet::hd::UnhardenedIndex;
use wallet::scripts::address::AddressCompat;

use crate::model::{
    file, DescriptorClass, DescriptorError, FileDocument, Signer, Wallet, WalletSettings,
};

pub(super) struct AddressRow {
    pub change: bool,
    pub index: UnhardenedIndex,
    pub address: AddressCompat,
    pub balance: u64,
}

impl AddressRow {
    pub fn insert_item(&self, store: &ListStore) -> TreeIter {
        store.insert_with_values(None, &[(0, &self.address.to_string()), (1, &self.balance)])
    }
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

    pub fn save(&mut self, path: impl AsRef<Path>) -> Result<usize, file::Error> {
        self.path = path.as_ref().to_owned();
        self.wallet.write_file(path)
    }

    pub fn as_wallet(&self) -> &Wallet {
        &self.wallet
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
    ) -> Result<(), DescriptorError> {
        self.wallet.update_signers(signers)?;
        for class in descriptor_classes {
            self.wallet.add_descriptor_class(class);
        }
        // TODO: Produce more addresses
        Ok(())
    }

    pub(super) fn generate_addresses(&self, change: bool, count: u16) -> Vec<AddressRow> {
        self.as_settings()
            .addresses(false, 0..=(count - 1))
            .expect("internal inconsistency in wallet descriptor")
            .into_iter()
            .map(|(index, address)| AddressRow {
                change,
                index,
                address,
                balance: 0,
            })
            .collect()
    }
}
