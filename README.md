# WFBuddy

A crossplatform tool and helper for Warframe.

WFBuddy looks at your game every 3(configurable) seconds, depending on what it sees, it'll display relevant info. PaddleOCR is used for the text recognition.

## Features
- **Cross platform**: Supporting both Windows and Linux.
- **Automatic detection**: No hotkeys required.
- **Relic rewards**: Easy overview of prime values from warframe.market.

## Planned
- **Other languages**: Currently only English interface is supported.
- **Ui Scale**: Currently only a ui scale of 100% is supported.
- **Overlay**: No more looking at an external window.

## Why
- **Why not look at the log file**: The log file is written to buffered. From testing sometimes taking more than 10 seconds to even detect the relic rewards screen is open, making it practically useless.
- **Why make another tool**: I wanted a tool that could run on Linux and had a gui.

## Disclaimer
WFBuddy is in no way associated with the game Warframe or the developer and publisher Digital Extremes.
While WFBuddy only gets screenshots from the game and does not interact with it in any other way, you are using this at your own risk.


## Build & run

Requirements:
- Rust toolchain (stable)

### Linux system dependencies

On Linux, `wfbuddy` depends on the `xcap` crate for window/screen capture. `xcap` uses native system
libraries (X11/Wayland/PipeWire) and needs the corresponding *development* packages installed.

Debian/Ubuntu:

```bash
sudo apt-get update
sudo apt-get install -y \
  pkg-config \
  libclang-dev \
  libxcb1-dev \
  libxrandr-dev \
  libdbus-1-dev \
  libpipewire-0.3-dev \
  libspa-0.2-dev \
  libwayland-dev \
  libegl-dev
```

From the repo root:

```bash
cargo run -p wfbuddy
```

### Configuration

WFBuddy stores config in your OS config directory under `WFBuddy/config.json`.

### Logging & debugging

Logging uses the standard `RUST_LOG` environment variable:

```bash
RUST_LOG=info cargo run -p wfbuddy
RUST_LOG=debug cargo run -p wfbuddy
```

OCR/debug helpers:
- `WFBUDDY_DEBUG_OCR=1` enables extra OCR debug logging.
- `WFBUDDY_WRITE_IMAGE=1` writes intermediate images to the current working directory (useful for debugging OCR issues).
