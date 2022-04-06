use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{PublicKey, SECP256K1};
use bitcoin::util::bip32::{ChainCode, ChildNumber, ExtendedPrivKey, ExtendedPubKey, Fingerprint};
use bitcoin::{secp256k1, Network};
use gtk::prelude::*;
use gtk::{Button, Dialog, ListStore, TextBuffer, ToggleButton, ToolButton, TreeView};
use relm::{Channel, Relm, Sender, StreamHandle, Update, Widget};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use gladis::Gladis;
use gtk::gdk::EventMask;
use hwi::error::Error as HwiError;
use hwi::HWIDevice;
use miniscript::descriptor::{DescriptorType, Sh, TapTree, Tr, Wsh};
use miniscript::policy::concrete::Policy;
use miniscript::{Descriptor, Legacy, Miniscript, Segwitv0, Tap};
use wallet::bitcoin_hd::{TerminalStep, XpubRef};
use wallet::hd::schemata::DerivationBlockchain;
use wallet::hd::{AccountStep, DerivationScheme, HardenedIndex, SegmentIndexes, TrackingAccount};
use wallet::scripts::taproot::TreeNode;

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

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum DescriptorClass {
    PreSegwit,
    SegwitV0,
    NestedV0,
    TaprootC0,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Model {
    pub scheme: DerivationScheme,
    pub devices: HardwareList,
    pub signers: BTreeSet<Signer>,
    pub network: PublicNetwork,
    pub descriptor: Option<Descriptor<TrackingAccount>>,
    pub class: DescriptorClass,
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
            network: PublicNetwork::Mainnet,
            descriptor: None,
            class: DescriptorClass::SegwitV0,
        }
    }
}

impl Model {
    pub fn update_devices(&mut self, devices: HardwareList) {
        self.devices = devices;
        self.update_signers()
    }

    pub fn update_signers(&mut self) {
        let known_xpubs = self
            .signers
            .iter()
            .map(|signer| signer.xpub)
            .collect::<BTreeSet<_>>();

        self.devices
            .iter()
            .filter(|(_, device)| !known_xpubs.contains(&device.default_xpub))
            .for_each(|(fingerprint, device)| {
                self.signers
                    .insert(Signer::with(*fingerprint, device.clone()));
            });
    }

    pub fn update_descriptor(&mut self) {
        let k = self.signers.len();
        let accounts = self
            .signers
            .iter()
            .map(|signer| {
                let path: Vec<ChildNumber> = self
                    .scheme
                    .to_account_derivation(signer.account.into(), self.network.into())
                    .into();
                TrackingAccount {
                    seed_based: true,
                    master: XpubRef::Fingerprint(signer.fingerprint),
                    account_path: path
                        .into_iter()
                        .map(AccountStep::try_from)
                        .collect::<Result<_, _>>()
                        .expect("inconsistency in constructed derivation path"),
                    account_xpub: signer.xpub,
                    revocation_seal: None,
                    terminal_path: vec![TerminalStep::Wildcard, TerminalStep::Wildcard],
                }
            })
            .collect::<Vec<_>>();

        let key_policies = accounts
            .iter()
            .map(|key| Policy::Key(key.clone()))
            .collect::<Vec<_>>();
        let sig_conditions = (1..=k)
            .into_iter()
            .map(|n| (n, Policy::Threshold(k, key_policies.clone())))
            .map(|(n, node)| {
                if n > 1 {
                    (
                        n,
                        Policy::And(vec![node, Policy::Older(10u32.pow(n as u32))]),
                    )
                } else {
                    (n, node)
                }
            })
            .collect::<Vec<_>>();

        let policy = Policy::Or(sig_conditions.clone());
        let ms_witscript = policy
            .compile::<Segwitv0>()
            .expect("policy composition  is broken");

        let wsh = Wsh::new(ms_witscript).expect("miniscript composition is broken");

        self.descriptor = Some(match self.class {
            DescriptorClass::PreSegwit => {
                let ms = policy
                    .compile::<Legacy>()
                    .expect("policy composition  is broken");
                Descriptor::Sh(Sh::new(ms).expect("miniscript composition is broken"))
            }
            DescriptorClass::SegwitV0 => Descriptor::Wsh(wsh),
            DescriptorClass::NestedV0 => Descriptor::Sh(Sh::new_with_wsh(wsh)),
            DescriptorClass::TaprootC0 => {
                let mut unspendable_key =
                    secp256k1::PublicKey::from_secret_key(&SECP256K1, &secp256k1::ONE_KEY);
                unspendable_key
                    .add_exp_assign(
                        &SECP256K1,
                        &sha256::Hash::hash(&unspendable_key.serialize()),
                    )
                    .unwrap();
                let mut buf = Vec::with_capacity(78);
                buf.extend(if self.network.is_testnet() {
                    [0x04u8, 0x35, 0x87, 0xCF]
                } else {
                    [0x04u8, 0x88, 0xB2, 0x1E]
                });
                buf.extend([0u8; 4]); // ver
                buf.extend([0u8; 5]); // depth + fingerprint
                buf.extend([0u8; 4]); // child no
                buf.extend(&unspendable_key.serialize());
                buf.extend(&unspendable_key.serialize());
                let unspendable_xkey =
                    ExtendedPubKey::decode(&buf).expect("broken unspendable key construction");
                let unspendable = TrackingAccount {
                    seed_based: true,
                    master: XpubRef::Unknown,
                    account_path: vec![],
                    account_xpub: unspendable_xkey,
                    revocation_seal: None,
                    terminal_path: vec![TerminalStep::Wildcard, TerminalStep::Wildcard],
                };

                let (tap_tree, remnant) = sig_conditions
                    .into_iter()
                    .map(|(depth, pol)| {
                        (
                            depth,
                            pol.compile::<Tap>()
                                .expect("tapscript construction is broken"),
                        )
                    })
                    .rfold(
                        (None, None)
                            as (
                                Option<TapTree<TrackingAccount>>,
                                Option<Miniscript<TrackingAccount, Tap>>,
                            ),
                        |(tree, prev), (depth, ms)| match (tree, prev) {
                            (None, None) if depth % 2 == 1 => (None, Some(ms)),
                            (None, None) if depth % 2 == 1 => {
                                (Some(TapTree::Leaf(Arc::new(ms))), None)
                            }
                            (None, Some(ms2)) => (
                                Some(TapTree::Tree(
                                    Arc::new(TapTree::Leaf(Arc::new(ms))),
                                    Arc::new(TapTree::Leaf(Arc::new(ms2))),
                                )),
                                None,
                            ),
                            (Some(tree), None) => (
                                Some(TapTree::Tree(
                                    Arc::new(TapTree::Leaf(Arc::new(ms))),
                                    Arc::new(tree),
                                )),
                                None,
                            ),
                            _ => unreachable!(),
                        },
                    );

                let tap_tree = tap_tree.or_else(|| remnant.map(|ms| TapTree::Leaf(Arc::new(ms))));

                Descriptor::Tr(
                    Tr::new(unspendable, tap_tree).expect("taproot construction is broken"),
                )
            }
        });
    }

    pub fn toggle_descr_class(&mut self, class: DescriptorClass) -> bool {
        if self.class == class {
            false
        } else {
            self.class = class;
            true
        }
    }
}

#[derive(Msg, Debug)]
pub(crate) enum Msg {
    Init(Arc<Mutex<Model>>),
    RefreshHw,
    HwRefreshed(Result<(HardwareList, Vec<Error>), Error>),
    ToggleDescr(DescriptorClass),
    Save,
    Cancel,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub(crate) struct Widgets {
    dialog: Dialog,
    signers_tree: TreeView,
    signers_store: ListStore,
    descriptor_buf: TextBuffer,
    descr_legacy_tgl: ToggleButton,
    descr_segwit_tgl: ToggleButton,
    descr_nested_tgl: ToggleButton,
    descr_taproot_tgl: ToggleButton,

    refresh_dlg: Dialog,

    save_btn: Button,
    cancel_btn: Button,
    refresh_btn: ToolButton,
    addsign_btn: ToolButton,
    removesign_btn: ToolButton,
}

impl Widgets {
    pub fn update_signers(&mut self, signers: &BTreeSet<Signer>) {
        let store = &mut self.signers_store;
        store.clear();
        for signer in signers {
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
    }

    pub fn update_descriptor(&mut self, descriptor: Option<&Descriptor<TrackingAccount>>) {
        let text = match descriptor {
            Some(descriptor) => format!("{}", descriptor),
            None => s!(""),
        };
        self.descriptor_buf.set_text(&text);
    }

    fn descr_class_toggle(&self, class: DescriptorClass) -> &ToggleButton {
        match class {
            DescriptorClass::PreSegwit => &self.descr_legacy_tgl,
            DescriptorClass::SegwitV0 => &self.descr_segwit_tgl,
            DescriptorClass::NestedV0 => &self.descr_nested_tgl,
            DescriptorClass::TaprootC0 => &self.descr_taproot_tgl,
        }
    }

    pub fn should_update_descr_class(&mut self, class: DescriptorClass) -> bool {
        self.descr_class_toggle(class).is_active()
    }

    pub fn update_descr_class(&mut self, class: DescriptorClass) {
        self.descr_legacy_tgl
            .set_active(class == DescriptorClass::PreSegwit);
        self.descr_segwit_tgl
            .set_active(class == DescriptorClass::SegwitV0);
        self.descr_nested_tgl
            .set_active(class == DescriptorClass::NestedV0);
        self.descr_taproot_tgl
            .set_active(class == DescriptorClass::TaprootC0);
    }
}

pub(crate) struct Win {
    model: Model,
    origin_model: Option<Arc<Mutex<Model>>>,
    channel: Channel<Msg>,
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
            }
            Msg::HwRefreshed(result) => {
                let devices = match result {
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

                self.model.update_devices(devices);
                self.widgets.update_signers(&self.model.signers);
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref());
                self.widgets.refresh_dlg.hide();
            }
            Msg::ToggleDescr(class) => {
                if self.widgets.should_update_descr_class(class)
                    && self.model.toggle_descr_class(class)
                {
                    self.widgets.update_descr_class(self.model.class);
                }
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
            widgets.descr_legacy_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::PreSegwit)
        );
        connect!(
            relm,
            widgets.descr_segwit_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::SegwitV0)
        );
        connect!(
            relm,
            widgets.descr_nested_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::NestedV0)
        );
        connect!(
            relm,
            widgets.descr_taproot_tgl,
            connect_clicked(_),
            Msg::ToggleDescr(DescriptorClass::TaprootC0)
        );

        let stream = relm.stream().clone();
        let (channel, sender) = Channel::new(move |msg| {
            stream.emit(msg);
        });
        let scheme = model.scheme.clone();
        widgets.refresh_btn.connect_clicked(move |_| {
            sender.send(Msg::RefreshHw);
            // TODO: This fixes the schema used in the wallet once and forever
            let scheme = scheme.clone();
            let sender = sender.clone();
            std::thread::spawn(move || {
                let result = HardwareList::enumerate(&scheme, model.network, HardenedIndex::zero());
                sender.send(Msg::HwRefreshed(result));
            });
        });

        widgets.dialog.show();

        Win {
            model,
            widgets,
            channel,
            origin_model: None,
        }
    }
}
