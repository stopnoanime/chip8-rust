# CHIP-8 Emulator

![Screenshot](./screenshot.png)
This is a CHIP-8 emulator written in Rust.

## Emulator (`emu`)

To run the emulator:

```bash
cargo run -- <rom_path>
```

### Keybindings

- `1-4`, `Q-R`, `A-F`, `Z-V`: Map to CHIP-8 keys
- `Escape`: Exit the emulator

## Debugger (`dbg`)

To run the debugger:

```bash
cargo run --bin dbg -- <rom_path>
```

### Keybindings

**Running Mode:**
- `Escape`: Pause execution
- `1-4`, `Q-R`, `A-F`, `Z-V`: Map to CHIP-8 keys (same as emulator)

**Paused Mode:**
- `Escape`: Quit the debugger
- `Enter`: Execute command
- `Up`/`Down`: Scroll output
- Type commands to interact with the debugger. Enter `help` to see the list of available commands.

## ROMs

You can find ROMs here: [CHIP-8 Archive](https://johnearnest.github.io/chip8Archive/). Make sure the rom is made for the chip8 platform.
