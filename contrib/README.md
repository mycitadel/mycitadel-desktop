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


# Speed up the build process by using a faster linker

Cargo allows changing the default linker to an alternative (faster) one.
This can make iterative development much more pleasant.

`lld` was tested and known to work, saving about 5s (from 8s to 3s) in debug
mode builds.

For your conveniance the `contrib/cargo-config.toml` template is prepared and
you can follow the instructions inside it.

See [rust perf book][rust-perf-book] or [an informative blog post][blog-post]
for more information about this and more way to possibly improve the build times.

[rust-perf-book]: https://nnethercote.github.io/perf-book/compile-times.html#linking
[blog-post]: https://endler.dev/2020/rust-compile-times/#switch-to-a-faster-linker
