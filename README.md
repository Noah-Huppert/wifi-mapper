# Wifi Mapper
Map wireless networks.

# Table Of Contents
- [Overview](#overview)
- [Install](#install)
- [Use](#use)

# Overview
Map wireless networks in an attempt to figure out their origin.

Uses a 3D coordinate system and associates wireless metadata with each node.

Outputs scanned nodes to a JSON file.

# Install
Wifi mapper is written in [Rust](https://www.rust-lang.org/).

To build:

```
% cargo build --release
```

This will produce the `target/release/wifi-mapper` binary.

# Use
Wifi mapper stores map information in a JSON file. Each invocation of the tool will add one node to the map.

Specify the map JSON file with the `-f` option.  

The tool may have to be run as a super user in order to have access to your wireless interface.

Run:

```
# wifi-mapper -f map-file.json
```
