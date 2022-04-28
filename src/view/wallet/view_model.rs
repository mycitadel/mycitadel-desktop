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

use super::pay::beneficiary_row::BeneficiaryModel;
use crate::model::{
    file, DescriptorClass, DescriptorError, ElectrumServer, FileDocument, Signer, Wallet,
    WalletSettings,
};

#[derive(Getters)]
pub struct ViewModel {
    wallet: Wallet,
    path: PathBuf,
    #[getter(as_mut)]
    beneficiaries: BeneficiaryModel,
    fee_rate: f32, // Used by payment window
}

impl ViewModel {
    pub fn with(wallet: Wallet, path: PathBuf) -> ViewModel {
        ViewModel {
            fee_rate: wallet.ephemerals().fees.0 * 100_000_000.0, // TODO: Update on window opening
            wallet,
            path,
            beneficiaries: BeneficiaryModel::new(),
        }
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
    ) -> Result<Option<&ElectrumServer>, DescriptorError> {
        self.wallet.update_signers(signers)?;
        for class in descriptor_classes {
            self.wallet.add_descriptor_class(class);
        }
        let electrum_updated = self.wallet.update_electrum(electrum);
        Ok(if electrum_updated {
            Some(self.wallet.as_settings().electrum())
        } else {
            None
        })
    }
}
