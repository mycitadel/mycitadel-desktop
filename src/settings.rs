use bitcoin::util::bip32::{ExtendedPubKey, Fingerprint};
use gtk::prelude::DialogExt;
use gtk::prelude::*;
use gtk::{Button, Dialog, DialogFlags, ResponseType, ToolButton};
use relm::{ContainerWidget, Relm, Update, Widget};
use std::collections::{BTreeMap, BTreeSet};
use std::ops::{Deref, DerefMut};
use std::sync::{Arc, Mutex};

use gladis::Gladis;
use wallet::hd::{DerivationScheme, HardenedIndex, SegmentIndexes};

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum Ownership {
    Mine,
    External,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug, Default)]
pub struct HardwareDevice {
    pub line: String,
    pub model: String,
}

#[derive(Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Signer {
    pub fingerprint: Fingerprint,
    pub device: Option<HardwareDevice>,
    pub name: String,
    pub xpub: ExtendedPubKey,
    pub account: HardenedIndex,
    pub ownership: Ownership,
}

#[derive(Clone, PartialEq, Eq, Hash, Debug)]
pub(crate) struct Model {
    pub scheme: DerivationScheme,
    pub devices: BTreeMap<Fingerprint, HardwareDevice>,
    pub signers: BTreeSet<Signer>,
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

                // self.widgets.refresh_dlg.hide();
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
