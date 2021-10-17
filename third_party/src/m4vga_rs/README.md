# Bits and pieces from Cliff Biffle's _excellent_ `m4vga-rs` library

The `m4vga-rs` repository ([github](https://github.com/cbiffle/m4vga-rs)) is not packaged into a crate, nor available on [crates.io](crates.io). A lot of the code was written back in 2018 when Rust was a lot younger, and in fact does not even compile with rust with the ["recent" change to use lld as the linker](https://twitter.com/rustembedded/status/1033100089009401857) (though passing the flag `-C linker=arm-none-eabi-ld` to use the old gcc linker _does_ compile). In addition, many of the crates used in the code are quite old and would take a non-zero amount of effort to port.

Instead, I've opted to bring in small bits and pieces in reusable chunks. Nearly everything in this directory is going to be copied out word for word and only modified when necessary to compile with more modern crate versions (in compliance with the [LICENSE](./LICENSE)). 