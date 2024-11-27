# RADICO
A command line streaming music player, written in Rust.

for research purpose only.

## BUILD
### ⚠ About building and running on Linux

This program uses [rustaudio/cpal](https://github.com/rustaudio/cpal) lib to play audio, which requires ALSA development files on Linux.

In order to build and run this program on Linux, you need to install：

- `libasound2-dev` on Debian / Ubuntu
- `alsa-lib-devel` on Fedora
- `alsa-lib-dev` on Alpine

### ⚠ About running on Windows

The program can also play audio using the [ASIO4ALL](https://asio4all.org) driver instead of WASAPI.

### ⚠ About building on Raspberry Pi

For Raspberry Pi model B v1.2, cross build using arm-unknown-linux-gnueabihf target.

## TODO
- [x] add station menu

## Usage

```
[Key]                [Description]
 0-9                  adjust volume
 i                    station info
 n                    next station
 p                    previous station
 Q                    quit
 Ctrl+C               exit

Usage: radico [-s] [--cert=<cert>] [url]

Available positional items:
    url                  url

Available options:
    -s, --show-dev-list  show device list
        --cert=<cert>    certificate
    -h, --help           Prints help information
```
## License
The source code is licensed MIT. The website content is licensed CC BY 4.0,see LICENSE.
