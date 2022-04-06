use bitcoin::secp256k1::PublicKey;
use bitcoin::util::bip32::{ChainCode, ExtendedPubKey, Fingerprint};
use bitcoin::Network;
use gtk::prelude::*;
use gtk::{Button, Dialog, ListStore, ToolButton, TreeView};
use relm::{Relm, Update, Widget};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use gladis::Gladis;
use hwi::error::Error as HwiError;
use hwi::HWIDevice;
use wallet::hd::schemata::DerivationBlockchain;
use wallet::hd::{DerivationScheme, HardenedIndex, SegmentIndexes};

// TODO: Move to descriptor wallet or BPro
#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Display)]
pub enum PublicNetwork {
    #[display("mainnet")]
    Mainnet,
    #[display("testnet")]
    Testnet,
    #[display("signet")]
    Signet,
}

impl From<PublicNetwork> for Network {
    fn from(network: PublicNetwork) -> Self {
        match network {
            PublicNetwork::Mainnet => Network::Bitcoin,
            PublicNetwork::Testnet => Network::Testnet,
            PublicNetwork::Signet => Network::Signet,
        }
    }
}

impl From<PublicNetwork> for DerivationBlockchain {
    fn from(network: PublicNetwork) -> Self {
        match network {
            PublicNetwork::Mainnet => DerivationBlockchain::Bitcoin,
            PublicNetwork::Testnet => DerivationBlockchain::Testnet,
            PublicNetwork::Signet => DerivationBlockchain::Testnet,
        }
    }
}

impl PublicNetwork {
    pub fn is_testnet(self) -> bool {
        matches!(self, PublicNetwork::Testnet | PublicNetwork::Signet)
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Ownership {
    Mine,
    External,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HardwareDevice {
    pub device_type: String,
    pub model: String,
    pub default_account: HardenedIndex,
    pub default_xpub: ExtendedPubKey,
}

#[derive(Wrapper, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default, From)]
pub struct HardwareList(BTreeMap<Fingerprint, HardwareDevice>);

#[derive(Debug, Display, Error)]
#[display(doc_comments)]
pub enum Error {
    /// No devices detected or some of devices are locked
    NoDevices(HwiError),

    /// Device {1} ({2}, master fingerprint {0} does not support used derivation schema {3} on {4}.
    DerivationNotSupported(
        Fingerprint,
        String,
        String,
        DerivationScheme,
        PublicNetwork,
        HwiError,
    ),
}

impl Error {
    pub fn into_hwi_error(self) -> HwiError {
        match self {
            Error::NoDevices(err) => err,
            Error::DerivationNotSupported(_, _, _, _, _, err) => err,
        }
    }
}

impl HardwareList {
    pub fn enumerate(
        scheme: &DerivationScheme,
        network: PublicNetwork,
        default_account: HardenedIndex,
    ) -> Result<(HardwareList, Vec<Error>), Error> {
        let mut devices = bmap![];
        let mut log = vec![];

        for device in HWIDevice::enumerate().map_err(Error::NoDevices)? {
            let fingerprint = Fingerprint::from(&device.fingerprint[..]);

            let derivation = scheme.to_account_derivation(default_account.into(), network.into());
            let derivation_string = derivation.to_string();
            match device.get_xpub(
                &derivation_string.parse().expect(
                    "ancient bitcoin version with different derivation path implementation",
                ),
                network.is_testnet(),
            ) {
                Ok(hwikey) => {
                    let xpub = ExtendedPubKey {
                        network: network.into(),
                        depth: hwikey.xpub.depth,
                        parent_fingerprint: Fingerprint::from(&hwikey.xpub.parent_fingerprint[..]),
                        child_number: u32::from(hwikey.xpub.child_number).into(),
                        public_key: PublicKey::from_slice(&hwikey.xpub.public_key.key.serialize())
                            .expect("secp lib used by hwi is broken"),
                        chain_code: ChainCode::from(&hwikey.xpub.chain_code[..]),
                    };
                    devices.insert(
                        fingerprint,
                        HardwareDevice {
                            device_type: device.device_type,
                            model: device.model,
                            default_account,
                            default_xpub: xpub,
                        },
                    );
                }
                Err(err) => {
                    log.push(Error::DerivationNotSupported(
                        fingerprint,
                        device.device_type,
                        device.model,
                        scheme.clone(),
                        network,
                        err,
                    ));
                }
            };
        }
        Ok((devices.into(), log))
    }
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Signer {
    pub fingerprint: Fingerprint,
    pub device: Option<String>,
    pub name: String,
    pub xpub: ExtendedPubKey,
    pub account: HardenedIndex,
    pub ownership: Ownership,
}

impl Signer {
    pub fn with(fingerprint: Fingerprint, device: HardwareDevice) -> Signer {
        Signer {
            fingerprint,
            device: Some(device.device_type),
            name: device.model.clone(),
            xpub: device.default_xpub,
            account: device.default_account,
            ownership: Ownership::Mine,
        }
    }
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Model {
    pub scheme: DerivationScheme,
    pub devices: HardwareList,
    pub signers: BTreeSet<Signer>,
    pub network: PublicNetwork,
}

impl Default for Model {
    fn default() -> Self {
        Model {
            // TODO: Add `ScriptType` to descriptor-wallet and simplify constructor
            scheme: DerivationScheme::Bip48 {
                script_type: HardenedIndex::from_index(2u32).unwrap(),
            },
            devices: none!(),
            signers: none!(),
            network: PublicNetwork::Testnet,
        }
    }
}

#[derive(Msg, Debug)]
pub(crate) enum Msg {
    Init(Arc<Mutex<Model>>),
    RefreshHw,
    Save,
    Cancel,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub(crate) struct Widgets {
    dialog: Dialog,
    signers_tree: TreeView,
    signers_store: ListStore,

    refresh_dlg: Dialog,

    save_btn: Button,
    cancel_btn: Button,
    refresh_btn: ToolButton,
    addsign_btn: ToolButton,
    removesign_btn: ToolButton,
}

pub(crate) struct Win {
    model: Model,
    origin_model: Option<Arc<Mutex<Model>>>,
    widgets: Widgets,
}

impl Update for Win {
    // Specify the model used for this widget.
    type Model = Model;
    // Specify the model parameter used to init the model.
    type ModelParam = Arc<Mutex<Model>>;
    // Specify the type of the messages sent to the update function.
    type Msg = Msg;

    fn model(relm: &Relm<Self>, model: Self::ModelParam) -> Self::Model {
        relm.stream().emit(Msg::Init(model.clone()));
        model.lock().expect("wallet model locked").deref().clone()
    }

    fn update(&mut self, event: Msg) {
        match event {
            Msg::Init(origin_model) => {
                self.origin_model = Some(origin_model);
            }
            Msg::RefreshHw => {
                self.widgets.refresh_dlg.show();

                self.model.devices = match HardwareList::enumerate(
                    &self.model.scheme,
                    self.model.network,
                    HardenedIndex::zero(),
                ) {
                    Err(_err) => {
                        // TODO: Display message to user
                        HardwareList::default()
                    }
                    Ok((devices, log)) if !log.is_empty() => {
                        // TODO: Display log and do not hide the window
                        devices
                    }
                    Ok((devices, log)) if devices.is_empty() => {
                        // TODO: Display message to user
                        devices
                    }
                    Ok((devices, _)) => devices,
                };

                let signers = &mut self.model.signers;
                let known_xpubs = signers
                    .iter()
                    .map(|signer| signer.xpub)
                    .collect::<BTreeSet<_>>();
                self.model
                    .devices
                    .0
                    .iter()
                    .filter(|(_, device)| !known_xpubs.contains(&device.default_xpub))
                    .for_each(|(fingerprint, device)| {
                        signers.insert(Signer::with(*fingerprint, device.clone()));
                    });

                let store = &mut self.widgets.signers_store;
                store.clear();
                for signer in &self.model.signers {
                    store.insert_with_values(
                        None,
                        &[
                            (0, &signer.name),
                            (1, &signer.fingerprint.to_string()),
                            (2, &signer.account.to_string()),
                            (3, &signer.xpub.to_string()),
                            (4, &signer.device.clone().unwrap_or_default()),
                        ],
                    );
                }

                self.widgets.refresh_dlg.hide();
            }
            Msg::Save => {
                self.origin_model.as_ref().map(|model| {
                    *(model.lock().expect("wallet model locked").deref_mut()) = self.model.clone();
                });
                self.widgets.dialog.hide();
            }
            Msg::Cancel => {
                self.widgets.dialog.hide();
            }
        }
    }
}

impl Widget for Win {
    // Specify the type of the root widget.
    type Root = Dialog;

    // Return the root widget.
    fn root(&self) -> Self::Root {
        self.widgets.dialog.clone()
    }

    fn view(relm: &Relm<Self>, model: Self::Model) -> Self {
        let glade_src = include_str!("../res/settings.glade");
        let widgets = Widgets::from_string(glade_src).unwrap();

        connect!(relm, widgets.save_btn, connect_clicked(_), Msg::Save);
        connect!(relm, widgets.cancel_btn, connect_clicked(_), Msg::Cancel);
        connect!(
            relm,
            widgets.refresh_btn,
            connect_clicked(_),
            Msg::RefreshHw
        );
        //connect!(relm, widgets.dialog, connect_destroy(_), Msg::Close);

        widgets.dialog.show();

        Win {
            model,
            widgets,
            origin_model: None,
        }
    }
}
