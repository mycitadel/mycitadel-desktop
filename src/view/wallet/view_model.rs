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
use std::path::{Path, PathBuf};

use bitcoin::Txid;
use bpro::{
    file, DescriptorError, ElectrumServer, FileDocument, HistoryEntry, Signer, Wallet,
    WalletSettings,
};
use wallet::descriptors::DescriptorClass;
use wallet::hd::UnhardenedIndex;
use crate::model::FormatDate;

use super::pay::beneficiary_row::BeneficiaryModel;
use crate::worker::exchange::{Exchange, Fiat};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Default)]
pub struct InvoiceModel {
    pub amount: Option<u64>,
    pub index: Option<UnhardenedIndex>,
}

#[derive(Getters)]
pub struct ViewModel {
    #[getter(as_mut)]
    wallet: Wallet,

    path: PathBuf,

    #[getter(as_mut)]
    beneficiaries: BeneficiaryModel,

    #[getter(as_copy)]
    fee_rate: f32, // Used by payment window

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

    pub fn as_settings(&self) -> &WalletSettings { self.wallet.as_settings() }
    pub fn to_settings(&self) -> WalletSettings { self.wallet.to_settings() }

    pub fn as_invoice(&self) -> &InvoiceModel { &self.invoice }
    pub fn as_invoice_mut(&mut self) -> &mut InvoiceModel { &mut self.invoice }

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

    pub fn export_history(&self, path: impl AsRef<Path>) {
        #[derive(Serialize, Deserialize)]
        #[serde(crate = serde_crate)]
        struct Entry {
            pub timestamp: String,
            pub height: u32,
            pub txid: Txid,
            pub label: String,
            pub amount: u64,
            pub balance: u64,
            pub fee: u64,
            pub fee_rate: u64,
        }

        impl From<HistoryEntry> for Entry {
            fn from(entry: HistoryEntry) -> Self { Entry {
                timestamp: entry.onchain.format_date(),
                height: entry.onchain.status.into_u32(),
                txid: entry.onchain.txid,
                label: entry.comment.map(|c| c.label).unwrap_or_default(),
                amount: entry.,
                balance: 0,
                fee: 0,
                fee_rate: 0,
            } }
        }

        let history: Vec<_> = self.model.wallet().history().iter().collect();
    }
}
