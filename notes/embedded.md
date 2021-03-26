Going off of [Embedded Rust](https://docs.rust-embedded.org/book/intro/index.html)

# Setup

- Tooling
  - `cargo-binutils`
  - `qemu-system-arm`
  - Install Cortex-M4F target support
    - `rustup target add thumbv7em-none-eabihf`
      - `hf` is hardware float
      - For M4F and M7F 
  - Template project install
    - `cargo install cargo-generate`
  - LLVM tools preview
    - `rustup component add llvm-tools-preview`
- Dev environment
  - `sudo apt install gdb-multiarch`
  - Udev rules in `/etc/udev/rules.d/49-stlinkv3.rules`
    ```
    # ST_PKG_VERSION 1.0.2-2
    # stlink-v3 boards (standalone and embedded) in usbloader mode and standard (debug) mode

    SUBSYSTEMS=="usb", ATTRS{idVendor}=="0483", ATTRS{idProduct}=="374d", \
        MODE="660", GROUP="plugdev", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1", \
        SYMLINK+="stlinkv3loader_%n"

    SUBSYSTEMS=="usb", ATTRS{idVendor}=="0483", ATTRS{idProduct}=="374e", \
        MODE="660", GROUP="plugdev", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1", \
        SYMLINK+="stlinkv3_%n"

    SUBSYSTEMS=="usb", ATTRS{idVendor}=="0483", ATTRS{idProduct}=="374f", \
        MODE="660", GROUP="plugdev", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1", \
        SYMLINK+="stlinkv3_%n"

    SUBSYSTEMS=="usb", ATTRS{idVendor}=="0483", ATTRS{idProduct}=="3753", \
        MODE="660", GROUP="plugdev", TAG+="uaccess", ENV{ID_MM_DEVICE_IGNORE}="1", \
        SYMLINK+="stlinkv3_%n"
    ```
    - Reload via `sudo udevadm control --reload-rules`
- Test openocd via `openocd -f interface/stlink.cfg -f target/stm32g4x.cfg`
  - Should listen on port 3333
  - Can also work with `interface/stlink-dap.cfg` for the `st-link` driver
    - More info [here under st-link](http://openocd.org/doc/html/Debug-Adapter-Configuration.html#st_005flink_005fdap_005finterface)
  - Need to use `-c "adapter speed 24000"` to set 24MHz
    - Note: config sets the following on `configure reset-start`, which might drop adapter speed when debugging
      ```
      $_TARGETNAME configure -event reset-start {
        # Reset clock is HSI (16 MHz)
        adapter speed 2000
      }
      ```

# First steps 

- Generate app from template `cortex-m-quickstart`
  - `cargo generate --git https://github.com/rust-embedded/cortex-m-quickstart`
  - Had to update various bits from `thumbv7m-none-eabi` to `thumbv7em-none-eabihf`
- Can retarget by `cargo build --target thumbv7-whatever`
- Updated linker file `memory.x`
  - CCMRAM set to be at 0x1000_0000
- `#![no_std]` stops from linking to main
- `#![no_main]` won't use standard `fn main()`
  - Apparently this requires nightlies...? On nightlies, so gotta figure that out
- Multiple panic handlers
  - `use panic_halt as _;` - Put a breakpoint on `rust_begin_unwind` to catch panics
  - `use panic_abort as _;` - Requires nightly
    - Test out
  - `use panic_itm as _;` - Panics out ITM (if available)
  - `use panic_semihosting as _;` - Panics out stderr, but requires a dbugger
- Entry point defined by `use cortex_m_rt::entry;` attribute (decorator)
- Inspect output with `cargo-binutils`
  - `cargo readobj --bin app -- -file-headers`
  - `cargo size --release --bin app -- -A -x`
    - Shows section locatin and sizes
    - ELFs aren't actual binaries; contain more info than actually flashed
      - Size will be different
  - `cargo objdump --bin app --release -- --disassemble --no-show-raw-insn --print-imm-hex`
  - Note: to pipe to code use hyphenated `cargo-foo` binaries

## Qemu

- Skipping for now; stm32 support isn't there unelss you use [qemu_stm32](https://github.com/beckus/qemu_stm32)