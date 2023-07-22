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
use std::convert::Infallible;
use std::path::PathBuf;
use std::str::FromStr;

use bpro::{file, DescriptorError, ElectrumServer, FileDocument, Signer, Wallet, WalletSettings};
use gtk::glib::ObjectExt;
use gtk::prelude::ListModelExt;
use rgb::BlockchainResolver;
use rgbstd::containers::{Bindle, BindleParseError, Contract};
use rgbstd::contract::ContractId;
use rgbstd::interface::rgb20::Rgb20;
use rgbstd::interface::FungibleAllocation;
use rgbstd::persistence::{Inventory, InventoryError};
use rgbstd::validation;
use wallet::descriptors::DescriptorClass;
use wallet::hd::UnhardenedIndex;

use super::asset_row::{AssetInfo, AssetModel};
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

    asset: Option<ContractId>,

    #[getter(as_mut)]
    beneficiaries: BeneficiaryModel,

    #[getter(as_mut)]
    asset_model: AssetModel,

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

#[derive(Debug, Display, Error, From)]
#[display(inner)]
pub enum RgbImportError {
    #[from]
    Bindle(BindleParseError<ContractId>),
    #[from]
    Import(InventoryError<Infallible>),
    /// the provided contract doesn't implement RGB20 interface.
    #[display(doc_comments)]
    NotRgb20,
    /// invalid contract.
    ///
    /// {0}
    InvalidContract(validation::Status),
}

impl ViewModel {
    pub fn with(mut wallet: Wallet, path: PathBuf) -> ViewModel {
        let asset_model = AssetModel::new();
        if let Some(rgb_controller) = wallet.rgb_mut() {
            for iface in rgb_controller
                .contracts_with_iface("RGB20")
                .expect("internal RGB data inconsistency")
            {
                let iface = Rgb20::from(iface);
                let spec = iface.spec();
                asset_model.append(&AssetInfo::with(
                    spec,
                    iface.created(),
                    iface.balance(&mut wallet),
                    iface.contract_id(),
                ));
            }
        }

        ViewModel {
            fee_rate: wallet.ephemerals().fees.0 * 100_000_000.0, // TODO: Update on window opening
            wallet,
            path,
            asset: None,
            beneficiaries: BeneficiaryModel::new(),
            invoice: none!(),
            exchange: Exchange::Kraken,
            fiat: Fiat::CHF,
            exchange_rate: 0.0,
            asset_model,
        }
    }

    pub fn save(&mut self) -> Result<usize, file::Error> { self.wallet.write_file(&self.path) }

    pub fn as_settings(&self) -> &WalletSettings { self.wallet.as_settings() }
    pub fn to_settings(&self) -> WalletSettings { self.wallet.to_settings() }

    pub fn as_invoice(&self) -> &InvoiceModel { &self.invoice }
    pub fn as_invoice_mut(&mut self) -> &mut InvoiceModel { &mut self.invoice }

    pub fn asset_info(&mut self) -> AssetInfo {
        match self.asset {
            None => AssetInfo::placeholder(),
            Some(contract_id) => self.asset_info_for(contract_id),
        }
    }

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

    pub fn change_asset(&mut self, index: u32) -> bool {
        let Some(asset) = self.asset_model().item(index) else {
            return false;
        };
        let id = asset.property::<String>("contract");
        match id.as_str() {
            "-" => self.asset = None,
            id => {
                let id = ContractId::from_str(id).expect("invalid RGB contract");
                self.asset = Some(id);
            }
        }
        true
    }

    pub fn import_rgb_contract(
        &mut self,
        text: String,
        resolver: &mut BlockchainResolver,
    ) -> Result<validation::Status, RgbImportError> {
        let rgb = self
            .wallet
            .rgb_mut()
            .expect("calling RGB-specific method on non-RGB-enabled wallet");

        let contract = Bindle::<Contract>::from_str(&text)?;
        let id = contract.id();

        let contract = contract.unbindle().validate(resolver).map_err(|c| {
            RgbImportError::InvalidContract(c.validation_status().expect("validated").clone())
        })?;
        let status = rgb.import_contract(contract, resolver)?;

        let info = self.asset_info_for(id);
        self.asset_model.append(&info);

        Ok(status)
    }

    fn asset_for(&mut self, id: ContractId) -> Rgb20 {
        let rgb = self
            .wallet
            .rgb_mut()
            .expect("calling RGB-specific method on non-RGB-enabled wallet");

        let iface = rgb
            .contract_iface_named(id, "RGB20")
            .expect("Not an RGB20 contract");
        Rgb20::from(iface)
    }

    pub fn asset_allocations(&mut self) -> Vec<FungibleAllocation> {
        let Some(id) = self.asset else {
            return vec![];
        };
        let iface = self.asset_for(id);
        iface.allocations(&self.wallet).into_inner()
    }

    fn asset_info_for(&mut self, id: ContractId) -> AssetInfo {
        let iface = self.asset_for(id);
        let spec = iface.spec();
        AssetInfo::with(
            spec,
            iface.created(),
            iface.balance(&self.wallet),
            iface.contract_id(),
        )
    }
}
