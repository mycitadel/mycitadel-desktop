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

use relm::Sender;
use std::sync::mpsc;
use std::thread::JoinHandle;
use std::time::Duration;
use std::{io, thread};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub enum Exchange {
    #[display("api.kraken.com")]
    Kraken,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub enum Fiat {
    #[display("USDBTC")]
    USD,

    #[display("EURBTC")]
    EUR,

    #[display("CHFBTC")]
    CHF,
}

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
enum Cmd {
    Refresh,
    SetExchange(Exchange),
    SetFiat(Fiat),
}

#[derive(Clone, PartialOrd, PartialEq, Debug)]
pub enum Msg {
    Rate(Fiat, Exchange, f64),
    ChannelDisconnected,
    Error(String),
}

pub struct ExchangeWorker {
    worker_thread: JoinHandle<()>,
    watcher_thread: JoinHandle<()>,
    tx: mpsc::Sender<Cmd>,
}

impl ExchangeWorker {
    pub fn with(
        sender: Sender<Msg>,
        mut exchange: Exchange,
        mut fiat: Fiat,
        interval: u64,
    ) -> Result<Self, io::Error> {
        let (tx, rx) = mpsc::channel::<Cmd>();
        let worker_thread = thread::Builder::new()
            .name(s!("exchange"))
            .spawn(move || loop {
                let _ = match rx.recv() {
                    Ok(Cmd::Refresh) => exchange_refresh(exchange, fiat, &sender),
                    Ok(Cmd::SetExchange(e)) => {
                        exchange = e;
                        exchange_refresh(exchange, fiat, &sender)
                    }
                    Ok(Cmd::SetFiat(f)) => {
                        fiat = f;
                        exchange_refresh(exchange, fiat, &sender)
                    }
                    Err(_) => {
                        sender
                            .send(Msg::ChannelDisconnected)
                            .expect("exchange channel is broken");
                        Ok(())
                    }
                }
                .map_err(|err| {
                    sender
                        .send(Msg::Error(err))
                        .expect("exchange channel is broken");
                });
            })?;

        let sender = tx.clone();
        let watcher_thread = thread::Builder::new()
            .name(s!("xchng-timer"))
            .spawn(move || loop {
                thread::sleep(Duration::from_secs(interval));
                sender.send(Cmd::Refresh).expect("Exchange thread is dead")
            })
            .expect("unable to start exchange rate refreshing thread");

        Ok(ExchangeWorker {
            tx,
            worker_thread,
            watcher_thread,
        })
    }

    pub fn refresh(&self) {
        self.cmd(Cmd::Refresh)
    }

    pub fn set_exchange(&self, exchange: Exchange) {
        self.cmd(Cmd::SetExchange(exchange))
    }

    pub fn set_fiat(&self, fiat: Fiat) {
        self.cmd(Cmd::SetFiat(fiat))
    }

    fn cmd(&self, cmd: Cmd) {
        self.tx.send(cmd).expect("Exchange thread is dead")
    }
}

fn exchange_refresh(exchange: Exchange, fiat: Fiat, sender: &Sender<Msg>) -> Result<(), String> {
    unimplemented!()
}
