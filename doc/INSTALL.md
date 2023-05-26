Prerequisites
=============

## HWI

If you plan to work with hardware wallets it is required to get `hwi`
application installed and working (this application provides an 
interface to hardware wallets). In order to do that please follow 
[the instructions from the official repository][hwi].

One of the simplest ways to install it on Linux and macOS should be 
running
```console
$ sudo pip3 install hwi
```

Note that you need to have Python 3 and PIP installed. On Debian-based
systems this can be achieved through
```console
$ sudo apt install python3 python3-pip
```
For other systems, please refer to the python website for the 
installation instructions.

To see your hardware wallets you may need to modify udev rules; please
instruct hardware wallet documentation. The rule of thumb: if your device
is not seen with the original wallet companion app and doesn't appear in
`hwi enumerate` output, it won't be seen by MyCitadel either.


Using package managers
======================

## Debian package

This is the recommended way of installing MyCitadel for Debian systems.

You need to pick up a proper deb file matching your distribution. For
instance, for distributions based on the current Debian stable (like PureOS) 
please use `mycitadel_*_debian11_amd64.deb` files; for more frequently updated
distributions (like Ubuntu) try `mycitadel_*_ubuntu_amd64.deb`. The
difference is that `debian11` version depends on Python 3.9, which is no 
longer available on more recent Ubuntu releases.

Download deb package from the release files matching your platform and run 
locally
```console
$ sudo apt install mycitadel_N_OS_amd64.deb
```
replacing `N` and `OS` in the package name with the filename you have 
downloaded (like `mycitadel_1.3.0_1_ubuntu_amd64.deb`) matching latest 
MyCitadel version and the target platform.


## Flatpak (all Linux distributions)

First, you have to get flatpak installed and connected to Flathub remote. 
While some Linux distributions are shipping with Flatpak and Flathub 
pre-installed (Debian), others may not have them (Ubuntu).

To install flatpak please follow instructions from the official 
[flatpak website](https://flatpak.org/setup/). Do not forget to add Flathub 
as a remote repository:

```console
$ flatpak remote-add --if-not-exists flathub https://flathub.org/repo/flathub.flatpakrepo
```

After that, download MyCitadel flatpak file and install with `flatpack 
install` command, providing the filename of the downloaded file as an argument.


## Nix

To be written


Compiling from source
=====================

As an alternative, you may want to compile the program from the source code. 
This allows not to use binaries downloaded from the internet, reducing the 
risk of fishing attacks. However, comparing to other options, this is pretty 
low-level task which may require more qualification.

Please pay attention that at this moment compilation from source doesn't
provide integration of MyCitadel with the desktop environment (icons, 
presence in application/startup menu, launcher etc). If you prefer to have a 
fully-integrated experience, please use package managers as described in the 
previous section


Prerequisites
-------------

Compilation from source requires you to install additional tools and 
developer libraries. This operation should be done only once and is OS-specific.

- For Debian Linux, please do
    ```console
    $ sudo apt update && \
      sudo apt install -y build-essentials git cargo pkg-config libgtk-3-dev python3-dev
    ```
  
    If you'd like to build debian package for your machine, please also install
    ```console
    $ sudo apt install -y debhelper dh-make
    ```

- For macOS, please do
    ```console
    $ brew install gtk+3 libadwaita adwaita-icon-theme libusb
    ```

- For Windows, you need to install Visual Studio C tools and MSYSY2-based GTK.
  To do so please follow instructions at
  <https://www.gtk.org/docs/installations/windows/#using-gtk-from-msys2-packages>.

Cargo is a package manager for rust programming language.

You should have it installed locally; if you are not sure please consult
[Rust programming language website][rust], providing necessary installation
instructions.


### Using crates.io

Once you have `cargo` installed run the following command:
```console
$ cargo install mycitadel-desktop
```

This will download the source code from https://crates.io, compile and 
install the latest MyCitadel desktop version on the local system. To run the 
application type in the command line `mycitadel` command, which will open 
the application window on your desktop.

If you are getting compilation errors, please try the following:

1. Update to the latest rust by running `rustup update`.
2. Try doing `cargo install --locked mycitadel-desktop`, which will prevent 
   from using upstream dependencies which may have broken semantic versioning
   in their recent updates.


### Using local repository

You may also clone MyCitadel git repository locally and run copilation from 
inside of it. This allows installing the latest nightly version from the 
master branch.

```console
$ git clone https://github.com/mycitadel/mycitadel-desktop
$ cd mycitadel-desktop
$ cargo install --path .
```

You may also compile a specific release tag; for that after the cloning
in the reposirory directory run, replacing `v1.3.0` with the desired version 
name:
```console
$ git checkout v1.3.0
$ cargo install --path .
```

If the build fails try to do the same things as described in the "Using 
crates.io" section above.


[hwi]: https://github.com/bitcoin-core/HWI
[rust]: https://rust-lang.org
