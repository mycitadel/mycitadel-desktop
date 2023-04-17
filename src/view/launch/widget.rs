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

use std::borrow::Cow;
use std::path::PathBuf;

use bpro::{Requirement, WalletTemplate};
use gladis::Gladis;
use gtk::gdk_pixbuf::Pixbuf;
use gtk::prelude::*;
use gtk::{
    Adjustment, ApplicationWindow, Button, Image, ListBox, ListBoxRow, Notebook,
    RecentChooserWidget, Switch,
};
use relm::Relm;
use wallet::descriptors::DescriptorClass;
use wallet::onchain::PublicNetwork;

use super::{Msg, Page};
use crate::view::{APP_ICON, APP_ICON_TOOL};

#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    logo_img: Image,
    about_btn: Button,
    pages: Notebook,
    hwcount_adj: Adjustment,
    taproot_swch: Switch,
    testnet_swch: Switch,
    rgb_swch: Switch,

    create_box: ListBox,
    watchonly_row: ListBoxRow,
    singlesig_row: ListBoxRow,
    hodling_row: ListBoxRow,
    multisig_row: ListBoxRow,
    company_row: ListBoxRow,
    custom_row: ListBoxRow,
    lightning_row: ListBoxRow,

    import_box: ListBox,
    open_box: ListBox,
    recent: RecentChooserWidget,
}

impl Widgets {
    pub fn show(&self, page: Option<Page>) {
        self.window.show();
        if let Some(page) = page {
            self.pages.set_current_page(Some(page as u32));
        }
    }

    pub fn hide(&self) { self.window.hide() }

    pub fn to_root(&self) -> ApplicationWindow { self.window.clone() }
    pub fn as_root(&self) -> &ApplicationWindow { &self.window }

    pub fn init_ui(&self) {
        let icon = Pixbuf::from_read(APP_ICON).expect("app icon is missed");
        self.window.set_icon(Some(&icon));

        let img = Pixbuf::from_read(APP_ICON_TOOL).expect("small app icon is missed");
        self.logo_img.set_pixbuf(Some(&img));
    }

    fn is_taproot(&self) -> bool { self.taproot_swch.is_active() }
    fn is_rgb(&self) -> bool { self.rgb_swch.is_active() }

    fn network(&self) -> PublicNetwork {
        match self.testnet_swch.is_active() {
            true => PublicNetwork::Testnet,
            false => PublicNetwork::Mainnet,
        }
    }

    pub fn template(&self, index: i32) -> WalletTemplate {
        let class = if self.is_taproot() {
            DescriptorClass::TaprootC0
        } else {
            DescriptorClass::SegwitV0
        };
        let network = self.network();
        match index {
            0 if self.is_rgb() => {
                debug_assert!(self.is_taproot());
                WalletTemplate::taproot_singlesig_rgb(network, false)
            }
            1 if self.is_rgb() => {
                debug_assert!(self.is_taproot());
                WalletTemplate::taproot_singlesig_rgb(network, true)
            }
            0 => WalletTemplate::singlesig(class, network, false, false),
            1 => WalletTemplate::singlesig(class, network, true, false),
            2 => WalletTemplate::hodling(class, network, 4, Requirement::Allow, Requirement::Allow),
            3 => {
                let count = self.hwcount_adj.value() as u16;
                WalletTemplate::multisig(
                    class,
                    network,
                    Some(count),
                    Requirement::Require,
                    Requirement::Deny,
                )
            }
            4 => WalletTemplate::multisig(
                class,
                network,
                None,
                Requirement::Allow,
                Requirement::Require,
            ),
            5 => WalletTemplate::multisig(
                class,
                network,
                None,
                Requirement::Allow,
                Requirement::Allow,
            ),
            6 => todo!("Lightning wallets"),
            _ => unreachable!("unknown template"),
        }
    }

    pub fn selected_recent(&self) -> Option<PathBuf> {
        self.recent
            .current_uri()
            .map(String::from)
            .map(|s| s.trim_start_matches("file://").to_owned())
            .and_then(|s| urlencoding::decode(&s).map(Cow::into_owned).ok())
            .map(PathBuf::from)
    }

    pub(super) fn connect(&self, relm: &Relm<super::Component>) {
        connect!(relm, self.about_btn, connect_clicked(_), Msg::About);
        connect!(
            relm,
            self.create_box,
            connect_row_activated(_, row),
            Msg::Template(row.index())
        );
        connect!(
            relm,
            self.import_box,
            connect_row_activated(_, _),
            Msg::Import
        );
        connect!(
            relm,
            self.rgb_swch,
            connect_changed_active(_),
            Msg::ToggleRgb
        );
        connect!(
            relm,
            self.taproot_swch,
            connect_changed_active(_),
            Msg::ToggleTaproot
        );
        connect!(relm, self.open_box, connect_row_activated(_, row), {
            if row.index() == 0 {
                Msg::Wallet
            } else {
                Msg::Psbt(None)
            }
        });
        connect!(relm, self.recent, connect_item_activated(_), Msg::Recent);
        connect!(
            relm,
            self.window,
            connect_delete_event(_, _),
            return (Some(Msg::Close), Inhibit(true))
        );
    }

    pub fn update_rgb(&self) {
        let rgb = self.is_rgb();
        if rgb {
            self.taproot_swch.set_active(true);
        }
        self.hodling_row.set_sensitive(!rgb);
        self.multisig_row.set_sensitive(!rgb);
        self.company_row.set_sensitive(!rgb);
        self.custom_row.set_sensitive(!rgb);
    }

    pub fn update_taproot(&self) {
        if !self.is_taproot() {
            self.rgb_swch.set_active(false);
        }
    }
}
