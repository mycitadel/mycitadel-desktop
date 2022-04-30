# MyCitadel Desktop

## Bitcoin, Lightning and RGB wallet

![Banner](assets/banner.jpg)

Wallet for bitcoin, digital assets and bitcoin finance (#BiFi) smart contracts.

Tool for reliable hodling (with inheritance options), corporate & organization 
use, current accounts with instant Lightning payments. Works with single- and 
multisig setups, based on hardware, air-gaped, cold and server-side hot key 
storage, involving arbitrary complex time-lock scripts (with miniscript) and 
wide interoperability (because of use of wallet descriptors). Taproot-enabled 
from day one, including multisig- and script-based taproot.

MyCitadel™ is a suite of software, hardware and Internet services focused on 
digital individual sovereignty and privacy. It includes mobile &amp; desktop 
cross-platform wallets, web-of-trust contact &amp; identity management app, 
end-to-end encrypted chat app, command-line tools, wallet runtime library and 
server-side node, which can be self-hosted, run on MyCitadel Box at home or in 
private MyCitadel cloud.

The application is written with rust programming language, GTK+ framework and 
set of bitcoin &amp; lightning rust libraries developed by LNP/BP Standards 
Association, including client-side-validation, descriptor wallet, BP, LNP and 
RGB libraries. MyCitadel node also contains embedded LNP &amp; RGB Nodes 
provided by the Association.


# Installation

## Compiling from sources

Compilation from sources requires rust language installed. This can be done
as described on <https://rust-lang.org>. 

First, you need to install prerequisites. This operation should be done only
once and OS-specific.

- For Debian Linux, please do
    ```console
    $ sudo apt update
    $ sudo apt install -y cargo libssl-dev pkg-config g++ cmake libgtk-3-dev \
      libusb-1.0-0-dev libudev-dev python3-dev
    ```

- For Mac OS, please do
    ```console
    $ brew install gtk3 libadwaita adwaita-icon-theme libcanberra-gtk-module \
      libcanberra-gtk3-module libusb
    ```

- For Windows, you need to install Visual Studio C tools and MSYSY2-based GTK.
  To do so please follow instructions at 
  <https://www.gtk.org/docs/installations/windows/#using-gtk-from-msys2-packages>.

If you plan to work with hardware wallets it is required to get `hwi` 
application installed and working (this is an interface to hardware wallets):
```console
$ pip3 install hwi ecdsa hidapi libusb1 mnemonic pbkdf2 pyaes typing-extensions
```

After that you can compile the latest release with this command:

```console
$ cargo install mycitadel-desktop --locked
```

Finally, run the wallet by typing in
```console
$ mycitadel
```

# License

This application is free software and distributed without any warranty under 
AGPL-3.0 License.

(C) 2022 Pandora Prime Sarl, Neuchatel, Switzerland.<br>
Some rights are reserved; for details please read the license agreement.

For business, partnership and other enquiries please write to 
<enquiries@mycitadel.io>.
