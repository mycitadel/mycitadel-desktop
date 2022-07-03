// MyCitadel desktop wallet: bitcoin & RGB wallet based on GTK framework.
//
// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime SA, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

use std::collections::BTreeSet;
use std::path::PathBuf;

use wallet::hd::UnhardenedIndex;

use super::pay::beneficiary_row::BeneficiaryModel;
use crate::model::{
    file, DescriptorClass, DescriptorError, ElectrumServer, FileDocument, Signer, Wallet,
    WalletSettings,
};
use crate::worker::exchange::{Exchange, Fiat};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
pub struct InvoiceModel {
    pub amount: Option<u64>,
    pub index: Option<UnhardenedIndex>,
}

#[derive(Getters)]
pub struct ViewModel {
    wallet: Wallet,

    path: PathBuf,

    #[getter(as_mut)]
    beneficiaries: BeneficiaryModel,

    #[getter(as_copy)]
    fee_rate: f32, // Used by payment window

    #[getter(as_copy)]
    vsize: f32,

    #[getter(skip)]
    invoice: InvoiceModel,

    #[getter(as_copy)]
    pub exchange: Exchange,

    #[getter(as_copy)]
    pub fiat: Fiat,

    #[getter(as_copy)]
    pub exchange_rate: f64,
}

impl ViewModel {
    pub fn with(wallet: Wallet, path: PathBuf) -> ViewModel {
        ViewModel {
            fee_rate: wallet.ephemerals().fees.0 * 100_000_000.0, // TODO: Update on window opening
            vsize: 0.0,
            wallet,
            path,
            beneficiaries: BeneficiaryModel::new(),
            invoice: none!(),
            exchange: Exchange::Kraken,
            fiat: Fiat::CHF,
            exchange_rate: 0.0,
        }
    }

    pub fn save(&mut self) -> Result<usize, file::Error> { self.wallet.write_file(&self.path) }

    pub fn as_wallet(&self) -> &Wallet { &self.wallet }
    pub fn as_wallet_mut(&mut self) -> &mut Wallet { &mut self.wallet }
    pub fn to_wallet(&self) -> Wallet { self.wallet.clone() }

    pub fn as_settings(&self) -> &WalletSettings { self.wallet.as_settings() }
    pub fn to_settings(&self) -> WalletSettings { self.wallet.to_settings() }

    pub fn as_invoice(&self) -> &InvoiceModel { &self.invoice }
    pub fn as_invoice_mut(&mut self) -> &mut InvoiceModel { &mut self.invoice }

    pub fn set_vsize(&mut self, vsize: f32) { self.vsize = vsize; }

    pub fn set_fee_rate(&mut self, fee_rate: f32) { self.fee_rate = fee_rate; }

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
