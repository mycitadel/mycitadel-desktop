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

use bitcoin::secp256k1::SECP256K1;
use bitcoin::{Address, PublicKey};
use gtk::prelude::*;
use gtk::{ListStore, TreeIter};
use wallet::hd::{Descriptor, SegmentIndexes, UnhardenedIndex};

use crate::model::{
    file, DescriptorClass, DescriptorError, FileDocument, Signer, Wallet, WalletDescriptor,
};

pub(super) struct AddressRow {
    pub address: Address,
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
        // TODO: Produce more addresses
        Ok(())
    }

    pub(super) fn generate_addresses(&self, count: u16) -> Vec<AddressRow> {
        let (descriptor, _) = self
            .as_descriptor()
            .descriptors_all()
            .expect("internal inconsistency in wallet descriptor");
        let len = Descriptor::<PublicKey>::derive_pattern_len(&descriptor)
            .expect("internal inconsistency in wallet descriptor");
        let mut pat = vec![UnhardenedIndex::zero(); len];

        (0u16..count)
            .map(|i| {
                pat[len - 1] = UnhardenedIndex::from(i);
                let address = Descriptor::<PublicKey>::address(&descriptor, &SECP256K1, &pat)
                    .expect("address derivation impossible");
                AddressRow {
                    address,
                    balance: 0,
                }
            })
            .collect()
    }
}
