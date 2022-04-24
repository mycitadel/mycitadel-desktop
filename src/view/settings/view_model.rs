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

use bitcoin::util::bip32::ExtendedPubKey;
use miniscript::Descriptor;
use wallet::hd::{Bip43, TerminalStep, TrackingAccount};

use super::spending_row::SpendingModel;
use crate::model::{
    file, DescriptorClass, DescriptorError, FileDocument, HardwareList, PublicNetwork, Signer,
    Wallet, WalletDescriptor, WalletTemplate,
};

pub struct ViewModel {
    path: PathBuf,

    pub descriptor_classes: BTreeSet<DescriptorClass>,
    pub support_multiclass: bool,
    pub network: PublicNetwork,
    pub signers: Vec<Signer>,
    pub spending_model: SpendingModel,

    // Data provided by the parent window
    pub template: Option<WalletTemplate>,
    pub export_lnpbp: bool,

    // Non-persisting / dynamic data for this window
    pub active_signer: Option<Signer>,
    pub devices: HardwareList,
    pub descriptor: Option<Descriptor<TrackingAccount>>,
}

impl Default for ViewModel {
    fn default() -> Self {
        ViewModel {
            path: PathBuf::default(),
            devices: none!(),
            signers: none!(),
            active_signer: None,
            spending_model: SpendingModel::new(),
            network: PublicNetwork::Mainnet,
            descriptor: None,
            template: None,
            descriptor_classes: bset![DescriptorClass::SegwitV0],
            support_multiclass: false,
            export_lnpbp: true,
        }
    }
}

impl TryFrom<&ViewModel> for WalletDescriptor {
    type Error = DescriptorError;

    fn try_from(model: &ViewModel) -> Result<Self, Self::Error> {
        WalletDescriptor::with(
            model.signers.clone(),
            model.spending_model.spending_conditions(),
            model.descriptor_classes.clone(),
            model.terminal_derivation(),
            model.network,
        )
    }
}

impl ViewModel {
    pub fn new(template: WalletTemplate, path: PathBuf) -> Result<ViewModel, file::Error> {
        let model = ViewModel {
            path,
            template: Some(template),
            ..default!()
        };
        model.save()?;
        Ok(model)
    }

    pub fn with(descr: WalletDescriptor, path: PathBuf) -> ViewModel {
        let descriptor_classes = descr.descriptor_classes().clone();
        ViewModel {
            path,
            support_multiclass: descriptor_classes.len() > 1,
            descriptor_classes,
            network: *descr.network(),
            signers: descr.signers().clone(),
            spending_model: SpendingModel::from(descr.spending_conditions()),

            export_lnpbp: true,
            template: None,
            active_signer: None,
            devices: empty!(),
            descriptor: None,
        }
    }

    pub fn save(&self) -> Result<Option<WalletDescriptor>, file::Error> {
        WalletDescriptor::try_from(self)
            .ok()
            .map(Wallet::with)
            .map(|wallet| {
                wallet.write_file(&self.path)?;
                Ok(wallet.into_descriptor())
            })
            .transpose()
    }

    pub fn path(&self) -> &Path {
        &self.path
    }
    pub fn filename(&self) -> String {
        self.path.display().to_string()
    }

    pub fn is_new_wallet(&self) -> bool {
        self.template.is_some()
    }

    pub fn bip43(&self) -> Bip43 {
        let class = self
            .descriptor_classes
            .iter()
            .next()
            .expect("dscriptor must always have at least a single class");
        class.bip43(self.signers.len())
    }

    pub fn terminal_derivation(&self) -> Vec<TerminalStep> {
        match self.support_multiclass {
            false => vec![TerminalStep::range(0u8, 1u8), TerminalStep::Wildcard],
            true => vec![
                TerminalStep::Wildcard,
                TerminalStep::Wildcard,
                TerminalStep::Wildcard,
                TerminalStep::Wildcard,
            ],
        }
    }

    pub fn signer_by(&self, xpub: ExtendedPubKey) -> Option<&Signer> {
        self.signers.iter().find(|signer| signer.xpub == xpub)
    }

    pub fn derivation_for(&self, signer: &Signer) -> TrackingAccount {
        signer.to_tracking_account(self.terminal_derivation())
    }

    pub fn replace_signer(&mut self, signer: Signer) -> bool {
        for s in &mut self.signers {
            if *s == signer {
                *s = signer;
                return true;
            }
        }
        return false;
    }

    pub fn update_signers(&mut self) {
        let known_xpubs = self
            .signers
            .iter()
            .map(|signer| signer.xpub)
            .collect::<BTreeSet<_>>();

        for (fingerprint, device) in self
            .devices
            .iter()
            .filter(|(_, device)| !known_xpubs.contains(&device.default_xpub))
        {
            self.signers.push(Signer::with_device(
                *fingerprint,
                device.clone(),
                &self.bip43(),
                self.network,
            ));
        }
    }

    pub fn toggle_descr_class(&mut self, class: DescriptorClass) -> bool {
        if self.support_multiclass {
            if self.descriptor_classes.contains(&class) {
                self.descriptor_classes.remove(&class)
            } else {
                self.descriptor_classes.insert(class)
            }
        } else {
            if self.descriptor_classes == bset![class] {
                false
            } else {
                self.descriptor_classes = bset![class];
                true
            }
        }
    }

    pub fn update_descriptor(&mut self) -> Result<(), String> {
        if self.signers.is_empty() {
            self.descriptor = None;
            return Err(s!("you need to add at least one signer"));
        }
        // TODO: Return error
        let descriptor = WalletDescriptor::try_from(self as &Self)
            .ok()
            .as_ref()
            .map(WalletDescriptor::descriptors_all)
            .transpose()
            .map_err(|err| err.to_string())?
            .map(|(d, _)| d);
        self.descriptor = descriptor;
        Ok(())
    }
}
