use std::collections::BTreeSet;
use std::ops::{Deref, DerefMut};
use std::str::FromStr;
use std::sync::{Arc, Mutex};

use gladis::Gladis;
use gtk::prelude::*;
use gtk::{
    glib, Adjustment, Button, Dialog, Entry, Image, Label, ListBox, ListStore, TextBuffer,
    ToggleButton, ToolButton, TreeView,
};
use relm::{init, Channel, Component, Relm, Update, Widget};

use bitcoin::hashes::{sha256, Hash};
use bitcoin::secp256k1::{self, SECP256K1};
use bitcoin::util::bip32::{ChildNumber, ExtendedPubKey, Fingerprint};
use miniscript::descriptor::{Sh, TapTree, Tr, Wsh};
use miniscript::policy::concrete::Policy;
use miniscript::{Descriptor, Legacy, Miniscript, Segwitv0, Tap};
use wallet::hd::{AccountStep, DerivationScheme, HardenedIndex, SegmentIndexes, TrackingAccount};
use wallet::hd::{TerminalStep, XpubRef};

use crate::{devices, spending_row};
use crate::spending_row::SpendingModel;
use crate::types::{
    DescriptorClass, HardwareDevice, HardwareList, PublicNetwork, Signer,
};

#[derive(Clone)]
pub struct Model {
    pub scheme: DerivationScheme,
    pub devices: HardwareList,
    pub signers: BTreeSet<Signer>,
    pub spendings: SpendingModel,
    pub network: PublicNetwork,
    pub descriptor: Option<Descriptor<TrackingAccount>>,
    pub class: DescriptorClass,
    pub format_lnpbp: bool,
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
            spendings: SpendingModel::new(),
            network: PublicNetwork::Mainnet,
            descriptor: None,
            class: DescriptorClass::SegwitV0,
            format_lnpbp: false,
        }
    }
}

impl Model {
    pub fn signer_by(&self, xpub: ExtendedPubKey) -> Option<&Signer> {
        self.signers.iter().find(|signer| signer.xpub == xpub)
    }

    pub fn derivation_for(&self, signer: &Signer) -> TrackingAccount {
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

        self.update_descriptor();
    }

    pub fn update_descriptor(&mut self) {
        if self.signers.is_empty() {
            self.descriptor = None;
            return;
        }

        let k = self.signers.len();
        let accounts = self
            .signers
            .iter()
            .map(|signer| self.derivation_for(signer))
            .collect::<Vec<_>>();

        let key_policies = accounts
            .iter()
            .map(|key| Policy::Key(key.clone()))
            .collect::<Vec<_>>();
        let sig_conditions = (1..=k)
            .into_iter()
            .map(|n| (n, Policy::Threshold(k - n + 1, key_policies.clone())))
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

        let (policy, remnant) = sig_conditions.iter().rfold(
            (None, None)
                as (
                    Option<Policy<TrackingAccount>>,
                    Option<Policy<TrackingAccount>>,
                ),
            |(acc, prev), (index, pol)| match (acc, prev) {
                (None, None) if index % 2 == 1 => (None, Some(pol.clone())),
                (None, None) => (Some(pol.clone()), None),
                (None, Some(prev)) => (
                    Some(Policy::Or(vec![(*index, pol.clone()), (*index + 1, prev)])),
                    None,
                ),
                (Some(acc), None) => (
                    Some(Policy::Or(vec![(*index, pol.clone()), (*index + 1, acc)])),
                    None,
                ),
                _ => unreachable!(),
            },
        );
        let policy = policy
            .or(remnant)
            .expect("zero signing accounts must be filtered");
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
                buf.extend([0u8; 5]); // depth + fingerprint
                buf.extend([0u8; 4]); // child no
                buf.extend(&unspendable_key.serialize()[1..]);
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
            self.update_descriptor();
            true
        }
    }
}

#[derive(Msg)]
pub enum Msg {
    Show,
    Init(Arc<Mutex<Model>>),
    Devices,
    AddDevice(Fingerprint, HardwareDevice),
    SignerSelect,
    ToggleDescr(DescriptorClass),
    ExportFormat(bool),
    Save,
    Cancel,
}

// Create the structure that holds the widgets used in the view.
#[derive(Clone, Gladis)]
pub struct Widgets {
    dialog: Dialog,
    save_btn: Button,
    cancel_btn: Button,

    devices_btn: ToolButton,
    addsign_btn: ToolButton,
    removesign_btn: ToolButton,
    signers_tree: TreeView,
    signers_store: ListStore,

    spending_list: ListBox,

    name_fld: Entry,
    fingerprint_fld: Entry,
    xpub_fld: Entry,
    account_adj: Adjustment,
    accfp_fld: Entry,
    derivation_fld: Entry,
    device_lbl: Label,
    device_img: Image,
    device_status_img: Image,
    seed_mine_tgl: ToggleButton,
    seed_extern_tgl: ToggleButton,

    descriptor_buf: TextBuffer,
    descr_legacy_tgl: ToggleButton,
    descr_segwit_tgl: ToggleButton,
    descr_nested_tgl: ToggleButton,
    descr_taproot_tgl: ToggleButton,
    export_core_tgl: ToggleButton,
    export_lnpbp_tgl: ToggleButton,
}

impl Widgets {
    pub fn update_signer_details(&self, details: Option<(&Signer, TrackingAccount)>) {
        if let Some((signer, ref derivation)) = details {
            self.name_fld.set_text(&signer.name);
            self.fingerprint_fld
                .set_text(&signer.fingerprint.to_string());
            self.xpub_fld.set_text(&signer.xpub.to_string());
            self.account_adj
                .set_value(signer.account.first_index() as f64);
            self.accfp_fld
                .set_text(&signer.xpub.fingerprint().to_string());
            self.derivation_fld.set_text(&derivation.to_string());
        }
        if let Some((device, model)) =
            details.and_then(|(s, _)| s.device.as_ref().map(|d| (d, &s.name)))
        {
            self.device_img.set_visible(true);
            self.device_status_img.set_visible(true);
            self.device_lbl.set_text(&format!("{} ({})", device, model));
        } else {
            self.device_img.set_visible(false);
            self.device_status_img.set_visible(false);
            self.device_lbl.set_text("Unknown");
        }
    }

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

    pub fn update_descriptor(
        &mut self,
        descriptor: Option<&Descriptor<TrackingAccount>>,
        format: bool,
    ) {
        let text = match (descriptor, format) {
            (Some(descriptor), false) => format!("{:#}", descriptor),
            (Some(descriptor), true) => format!("{}", descriptor),
            (None, _) => s!(""),
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

pub struct Win {
    model: Model,
    origin_model: Option<Arc<Mutex<Model>>>,
    widgets: Widgets,
    devices_win: Component<devices::Win>,
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
            Msg::Show => self.widgets.dialog.show(),
            Msg::Init(origin_model) => {
                self.origin_model = Some(origin_model);
            }
            Msg::Devices => {
                self.devices_win.emit(devices::Msg::Show);
            }
            Msg::AddDevice(fingerprint, device) => {
                self.model.devices.insert(fingerprint, device);
                self.model.update_signers();
                self.widgets.update_signers(&self.model.signers);
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
            }
            Msg::SignerSelect => {
                let signer = self
                    .widgets
                    .signers_tree
                    .selection()
                    .selected()
                    .map(|(list_model, iter)| list_model.value(&iter, 3))
                    .as_ref()
                    .map(glib::Value::get::<String>)
                    .transpose()
                    .expect("unable to get xpub value from tree column")
                    .as_deref()
                    .map(ExtendedPubKey::from_str)
                    .transpose()
                    .expect("invalid signer xpub")
                    .and_then(|xpub| self.model.signer_by(xpub));
                self.widgets
                    .update_signer_details(signer.map(|s| (s, self.model.derivation_for(s))));
            }
            Msg::ToggleDescr(class) => {
                if self.widgets.should_update_descr_class(class)
                    && self.model.toggle_descr_class(class)
                {
                    self.widgets.update_descr_class(self.model.class);
                    self.widgets
                        .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
                }
            }
            Msg::ExportFormat(lnpbp) => {
                self.model.format_lnpbp = lnpbp;
                self.widgets
                    .update_descriptor(self.model.descriptor.as_ref(), self.model.format_lnpbp);
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
        let widgets = Widgets::from_string(glade_src).expect("glade file broken");

        let stream = relm.stream().clone();
        let (_channel, sender) = Channel::new(move |msg| {
            stream.emit(msg);
        });

        let devices_win = init::<devices::Win>((model.scheme.clone(), model.network, sender))
            .expect("error in devices dialog");

        connect!(relm, widgets.save_btn, connect_clicked(_), Msg::Save);
        connect!(relm, widgets.cancel_btn, connect_clicked(_), Msg::Cancel);
        connect!(relm, widgets.devices_btn, connect_clicked(_), Msg::Devices);

        connect!(
            relm,
            widgets.signers_tree,
            connect_cursor_changed(_),
            Msg::SignerSelect
        );

        connect!(
            relm,
            widgets.export_core_tgl,
            connect_toggled(_),
            Msg::ExportFormat(false)
        );
        connect!(
            relm,
            widgets.export_lnpbp_tgl,
            connect_toggled(_),
            Msg::ExportFormat(true)
        );

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
        widgets
            .spending_list
            .bind_model(Some(&model.spendings), move |item| {
                spending_row::RowWidgets::init(stream.clone(), item)
            });

        Win {
            model,
            widgets,
            devices_win,
            origin_model: None,
        }
    }
}
