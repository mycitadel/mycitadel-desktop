// Written in 2022 by
//     Dr. Maxim Orlovsky <orlovsky@pandoraprime.ch>
//
// Copyright (C) 2022 by Pandora Prime Sarl, Switzerland.
//
// This software is distributed without any warranty. You should have received
// a copy of the AGPL-3.0 License along with this software. If not, see
// <https://www.gnu.org/licenses/agpl-3.0-standalone.html>.

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug)]
pub enum ElectrumSec {
    Tor,
    Tls,
    None,
}

#[derive(Clone, Ord, PartialOrd, Eq, PartialEq, Hash, Debug, Display)]
pub enum ElectrumServer {
    #[display("electrum.mycitadel.io")]
    MyCitadel(bool),

    #[display("electrum.blockstream.info")]
    Blockstream(bool),

    #[display("{0}:{1}")]
    Custom(String, u16, ElectrumSec),
}
