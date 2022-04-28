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

use std::borrow::Cow;
use std::path::PathBuf;

use gladis::Gladis;
use gtk::prelude::*;
use gtk::{Adjustment, ApplicationWindow, Button, ListBox, Notebook, RecentChooserWidget, Switch};
use relm::Relm;

use crate::model::{PublicNetwork, Requirement, WalletTemplate};

use super::Msg;

#[derive(Clone, Gladis)]
pub struct Widgets {
    window: ApplicationWindow,
    about_btn: Button,
    pages: Notebook,
    hwcount_adj: Adjustment,
    taproot_swch: Switch,
    testnet_swch: Switch,
    rgb_swch: Switch,
    create_box: ListBox,
    import_box: ListBox,
    open_box: ListBox,
    recent: RecentChooserWidget,
}

impl Widgets {
    pub fn show(&self) {
        self.window.show();
        self.pages.set_current_page(Some(0));
    }

    pub fn hide(&self) {
        self.window.hide()
    }

    pub fn to_root(&self) -> ApplicationWindow {
        self.window.clone()
    }
    pub fn as_root(&self) -> &ApplicationWindow {
        &self.window
    }

    fn is_taproot(&self) -> bool {
        self.taproot_swch.is_active()
    }

    fn network(&self) -> PublicNetwork {
        match self.testnet_swch.is_active() {
            true => PublicNetwork::Testnet,
            false => PublicNetwork::Mainnet,
        }
    }

    pub fn template(&self, index: i32) -> WalletTemplate {
        let taproot = self.is_taproot();
        let network = self.network();
        match index {
            0 => WalletTemplate::singlesig(taproot, network, false),
            1 => WalletTemplate::singlesig(taproot, network, true),
            2 => WalletTemplate::hodling(network, 4, Requirement::Allow, Requirement::Allow),
            3 => {
                let count = self.hwcount_adj.value() as u16;
                WalletTemplate::multisig(
                    network,
                    Some(count),
                    Requirement::Require,
                    Requirement::Deny,
                )
            }
            4 => WalletTemplate::multisig(network, None, Requirement::Allow, Requirement::Require),
            5 => WalletTemplate::multisig(network, None, Requirement::Allow, Requirement::Allow),
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
}
