# Compile with nix

The [nix](https://nixos.org/download.html) allows to setup deterministic environment for external dependencies on any Linux distributive.

Run this in the `contrib` folder to enter the env:
```
nix-shell
```

Next you can build the project as usual:
```
cd ..
cargo run --release --bin mycitadel
```
