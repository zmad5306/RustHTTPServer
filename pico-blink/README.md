# pico-blink

Rust firmware for a Raspberry Pi Pico that blinks the onboard LED.

The easiest way to install it is with a normal USB cable and the Pico's
BOOTSEL mode. A debug probe is optional.

## One-Time Setup

Install Rust, then add the Pico build target and the tools this project uses:

```sh
rustup target add thumbv6m-none-eabi
cargo install flip-link
cargo install elf2uf2-rs --locked
```

`flip-link` is the linker configured in `.cargo/config.toml`.
`elf2uf2-rs` converts the compiled firmware into the `.uf2` file that the Pico
bootloader accepts.

## Build the Firmware

From this directory:

```sh
cargo build --release
```

That creates this firmware ELF:

```text
target/thumbv6m-none-eabi/release/pico-blink
```

For a quick compile check while developing, use:

```sh
cargo build
```

## Install with USB BOOTSEL

Use this path if you only have a USB cable.

1. Convert the release build to a UF2 file:

   ```sh
   elf2uf2-rs target/thumbv6m-none-eabi/release/pico-blink pico-blink.uf2
   ```

2. Unplug the Pico.

3. Hold the Pico's `BOOTSEL` button.

4. While still holding `BOOTSEL`, plug the Pico into USB.

5. Release `BOOTSEL`.

6. Your computer should mount a drive named `RPI-RP2`.

7. Copy the firmware onto that drive:

   macOS:

   ```sh
   cp pico-blink.uf2 /Volumes/RPI-RP2/
   ```

   Linux:

   ```sh
   cp pico-blink.uf2 /media/$USER/RPI-RP2/
   ```

   Windows PowerShell:

   ```powershell
   Copy-Item .\pico-blink.uf2 E:\
   ```

   Replace `E:` with the drive letter Windows assigned to `RPI-RP2`.

After the copy finishes, the Pico reboots automatically and the onboard LED
should blink every half second.

## Install with a Debug Probe

Use this path if you have a SWD debug probe connected to the Pico.

Install `probe-rs`:

```sh
cargo install probe-rs-tools --locked
```

Then flash and run:

```sh
cargo run --release
```

The project already configures `cargo run` to use:

```text
probe-rs run --chip RP2040 --protocol swd
```

You can also use `cargo embed` with the included `Embed.toml`:

```sh
cargo embed --release
```

That flashes the firmware and enables RTT/defmt logging.

## Change the Blink Speed

Edit the two `delay.delay_ms(500)` calls in `src/main.rs`, then rebuild and
install again.
