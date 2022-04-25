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
use std::fmt::{self, Display, Formatter};
use std::path::{Path, PathBuf};

use bitcoin::util::bip32::ExtendedPubKey;
use electrum_client::Client as ElectrumClient;
use miniscript::Descriptor;
use relm::{Channel, StreamHandle};
use wallet::hd::{Bip43, DerivationStandard, TerminalStep, TrackingAccount};

use super::spending_row::SpendingModel;
use super::Msg;
use crate::model::{
    file, DescriptorClass, DescriptorError, ElectrumSec, ElectrumServer, FileDocument,
    HardwareList, PublicNetwork, Signer, Wallet, WalletSettings, WalletTemplate,
};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub enum ElectrumPreset {
    #[display("electrum.mycitadel.io")]
    MyCitadel,

    #[display("electrum.blockstream.info")]
    Blockstream,

    #[display("")]
    Custom,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub struct ElectrumModel {
    pub electrum_preset: ElectrumPreset,
    pub electrum_server: String,
    pub electrum_port: u16,
    pub electrum_sec: ElectrumSec,
}

impl From<ElectrumModel> for ElectrumServer {
    fn from(model: ElectrumModel) -> Self {
        ElectrumServer {
            sec: model.electrum_sec,
            server: model.host(),
            port: model.electrum_port,
        }
    }
}

impl From<ElectrumServer> for ElectrumModel {
    fn from(electrum: ElectrumServer) -> Self {
        let electrum_preset = match electrum.server.as_str() {
            "electrum.blockstream.info" => ElectrumPreset::Blockstream,
            "electrum.mycitadel.io" => ElectrumPreset::MyCitadel,
            _ => ElectrumPreset::Custom,
        };
        ElectrumModel {
            electrum_preset,
            electrum_server: electrum.server,
            electrum_port: electrum.port,
            electrum_sec: electrum.sec,
        }
    }
}

impl Display for ElectrumModel {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "{}://{}:{}",
            self.electrum_sec,
            self.host(),
            self.electrum_port
        )
    }
}

impl ElectrumModel {
    fn new(network: PublicNetwork) -> Self {
        ElectrumModel {
            electrum_preset: ElectrumPreset::Blockstream,
            electrum_server: ElectrumPreset::Blockstream.to_string(),
            electrum_port: network.electrum_port(),
            electrum_sec: ElectrumSec::Tls,
        }
    }

    fn host(&self) -> String {
        if self.electrum_preset == ElectrumPreset::Custom {
            self.electrum_server.clone()
        } else {
            self.electrum_preset.to_string()
        }
    }
}

pub struct ViewModel {
    path: PathBuf,
    stream: StreamHandle<Msg>,

    pub descriptor_classes: BTreeSet<DescriptorClass>,
    pub support_multiclass: bool,
    pub network: PublicNetwork,
    pub signers: Vec<Signer>,
    pub spending_model: SpendingModel,
    pub electrum_model: ElectrumModel,

    // Data provided by the parent window
    pub template: Option<WalletTemplate>,
    pub export_lnpbp: bool,

    // Non-persisting / dynamic data for this window
    pub active_signer: Option<Signer>,
    pub devices: HardwareList,
    pub descriptor: Option<Descriptor<TrackingAccount>>,
}

impl TryFrom<&ViewModel> for WalletSettings {
    type Error = DescriptorError;

    fn try_from(model: &ViewModel) -> Result<Self, Self::Error> {
        WalletSettings::with(
            model.signers.clone(),
            model.spending_model.spending_conditions(),
            model.descriptor_classes.clone(),
            model.terminal_derivation(),
            model.network,
            model.electrum_model.clone().into(),
        )
    }
}

impl ViewModel {
    pub fn new(stream: StreamHandle<Msg>) -> Self {
        ViewModel {
            path: PathBuf::default(),
            stream,
            devices: none!(),
            signers: none!(),
            active_signer: None,
            spending_model: SpendingModel::new(),
            electrum_model: ElectrumModel::new(PublicNetwork::Mainnet),
            network: PublicNetwork::Mainnet,
            descriptor: None,
            template: None,
            descriptor_classes: bset![DescriptorClass::SegwitV0],
            support_multiclass: false,
            export_lnpbp: true,
        }
    }

    pub fn with_template(
        stream: StreamHandle<Msg>,
        template: WalletTemplate,
        path: PathBuf,
    ) -> Result<ViewModel, file::Error> {
        let model = ViewModel {
            path,
            stream,
            descriptor_classes: template
                .format
                .descriptor_types()
                .iter()
                .map(DescriptorClass::from)
                .collect(),
            support_multiclass: false,
            network: template.network,
            signers: empty!(),
            spending_model: template.conditions.clone().into(),
            electrum_model: ElectrumModel::new(template.network),
            template: Some(template),

            export_lnpbp: false,
            active_signer: None,
            devices: empty!(),
            descriptor: None,
        };
        model.save()?;
        Ok(model)
    }

    pub fn with_descriptor(
        stream: StreamHandle<Msg>,
        descr: WalletSettings,
        path: PathBuf,
    ) -> ViewModel {
        let descriptor_classes = descr.descriptor_classes().clone();
        ViewModel {
            path,
            stream,
            support_multiclass: descriptor_classes.len() > 1,
            descriptor_classes,
            network: *descr.network(),
            signers: descr.signers().clone(),
            spending_model: SpendingModel::from(descr.spending_conditions()),
            electrum_model: ElectrumModel::new(*descr.network()),

            export_lnpbp: true,
            template: None,
            active_signer: None,
            devices: empty!(),
            descriptor: None,
        }
    }

    pub fn stream(&self) -> StreamHandle<Msg> {
        self.stream.clone()
    }

    pub fn save(&self) -> Result<Option<WalletSettings>, file::Error> {
        WalletSettings::try_from(self)
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
            return Err(s!("You need to add at least one signer"));
        }
        let descriptor = WalletSettings::try_from(self as &Self)
            .ok()
            .as_ref()
            .map(WalletSettings::descriptors_all)
            .transpose()
            .map_err(|err| err.to_string())?
            .map(|(d, _)| d);
        self.descriptor = descriptor;
        Ok(())
    }

    pub fn test_electrum(&self) {
        enum ElectrumMsg {
            Ok,
            Failure(String),
        }
        let stream = self.stream.clone();
        let url = self.electrum_model.to_string();
        let (_channel, sender) = Channel::new(move |msg| match msg {
            ElectrumMsg::Ok => stream.emit(Msg::ElectrumTestOk),
            ElectrumMsg::Failure(err) => stream.emit(Msg::ElectrumTestFailed(err)),
        });
        let config = electrum_client::ConfigBuilder::new()
            .timeout(Some(5))
            .expect("we do not use socks here")
            .build();
        std::thread::spawn(move || match ElectrumClient::from_config(&url, config) {
            Err(err) => {
                sender
                    .send(ElectrumMsg::Failure(err.to_string()))
                    .expect("channel broken");
            }
            Ok(_) => {
                sender.send(ElectrumMsg::Ok).expect("channel broken");
            }
        });
    }
}
