# studi - Apple Studio Display Brightness Control

This tool allows for changing the brightness of one or more Apple Studio Display(s) attached to a Linux or Windows PC.

*Forked from `juliuszint/asdbctl` in March 2025, with added UI*

## Getting Started

A **compiled Windows executable** is attached to [release notes](https://github.com/himbeles/studi/releases/tag/v1.0.1).

Alternatively, compile using Rust:

```bash
git clone https://github.com/himbeles/studi.git && cd studi
# either just run from here
cargo run --release -- help
# or install with
cargo install --path .
```

## Usage

As a command line utility:

```sh
Tool to get or set the brightness for Apple Studio Displays. Launches UI if no command is given.

Usage: studio_display_brightness [OPTIONS] [COMMAND]

Commands:
  get   Get the current brightness in %
  set   Set the current brightness in %
  up    Increase the brightness
  down  Decrease the brightness
  help  Print this message or the help of the given subcommand(s)

Options:
  -s, --serial <SERIAL>  Serial number of the display for which to adjust the brightness
  -v, --verbose...       Turn debugging information on
  -h, --help             Print help
```

It offers a UI if launched without any subcommands.

## Troubleshooting

In case regular users are not allowed to access raw HID devices, you can use the udev rule in the `rules.d` directory.

## Technical Details

Dumping the USB traffic on macOS with Wireshark and setting the proper filters will show the USB Control transfers to set the brightness:

```sh
bmRequestType: 0x21
bRequest     : 0x9
wValue       : 0x0301
wIndex       : 0x000c
wLength      : 0x7
```

With this data package when setting it to the minimum brightness value:

```sh
[ 0x01, 0x90, 0x01, 0x00, 0x00, 0x00, 0x00 ]
```

The `0x90` and `0x01` are the brightness values encoded with the least significant byte first (LSB).

It is possible to operate the Studio Display in 3 different USB configurations, and Linux will use the first. This means that the USB interface number for controlling the brightness is not `0xc` (extracted from the dump) but `0x7`.

## Decoded HID Report Descriptor

```sh
Apple Studio Display HID report descriptor

# device 0:0
# 0x05, 0x80,                    // Usage Page (Monitor)                0
# 0x09, 0x01,                    // Usage (Monitor Control)             2
# 0xa1, 0x01,                    // Collection (Application)            4
# 0x85, 0x01,                    //  Report ID (1)                      6
# 0x06, 0x82, 0x00,              //  Usage Page (VESA Virtual Controls) 8
# 0x09, 0x10,                    //  Usage (Brightness)                 11
# 0x16, 0x90, 0x01,              //  Logical Minimum (400)              13
# 0x27, 0x60, 0xea, 0x00, 0x00,  //  Logical Maximum (60000)            16
# 0x67, 0xe1, 0x00, 0x00, 0x01,  //  Unit (SILinear: cm⁻² * cd)         21
# 0x55, 0x0e,                    //  Unit Exponent (-2)                 26
# 0x75, 0x20,                    //  Report Size (32)                   28
# 0x95, 0x01,                    //  Report Count (1)                   30
# 0xb1, 0x42,                    //  Feature (Data,Var,Abs,Null)        32

# 0x05, 0x0f,                    //  Usage Page (Vendor Usage Page 0x0f) 34
# 0x09, 0x50,                    //  Usage (Vendor Usage 0x50)          36
# 0x15, 0x00,                    //  Logical Minimum (0)                38
# 0x26, 0x20, 0x4e,              //  Logical Maximum (20000)            40
# 0x66, 0x10, 0x01,              //  Unit (None)                        43
# 0x55, 0x0d,                    //  Unit Exponent (-3)                 46
# 0x75, 0x10,                    //  Report Size (16)                   48
# 0xb1, 0x42,                    //  Feature (Data,Var,Abs,Null)        50

# 0x06, 0x82, 0x00,              //  Usage Page (VESA Virtual Controls) 52
# 0x09, 0x10,                    //  Usage (Brightness)                 55
# 0x16, 0x90, 0x01,              //  Logical Minimum (400)              57
# 0x27, 0x60, 0xea, 0x00, 0x00,  //  Logical Maximum (60000)            60
# 0x67, 0xe1, 0x00, 0x00, 0x01,  //  Unit (SILinear: cm⁻² * cd)         65
# 0x55, 0x0e,                    //  Unit Exponent (-2)                 70
# 0x75, 0x20,                    //  Report Size (32)                   72
# 0x95, 0x01,                    //  Report Count (1)                   74
# 0x81, 0x02,                    //  Input (Data,Var,Abs)               76

# 0xc0,                          // End Collection                      78
```

```
R: 79 05 80 09 01 a1 01 85 01 06 82 00 09 10 16 90 01 27 60 ea 00 00 67 e1 00 00 01 55 0e 75 20 95 01 b1 42 05 0f 09 50 15 00 26 20 4e 66 10 01 55 0d 75 10 b1 42 06 82 00 09 10 16 90 01 27 60 ea 00 00 67 e1 00 00 01 55 0e 75 20 95 01 81 02 c0
N: device 0:0
I: 3 0001 0001
```

## Related Projects

Other projects that helped along the way:

- [LG-ultrafine-brightness](https://github.com/ycsos/LG-ultrafine-brightness)
- [acdcontrol](https://github.com/yhaenggi/acdcontrol)