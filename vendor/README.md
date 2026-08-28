# Vendored Google Well-Known Types

This directory contains a vendored copy of the Protocol Buffers
well-known types (`google.protobuf` package) so that `cargo build` does not
depend on the WKT files shipped with the system `protoc` installation.

Source: <https://github.com/protocolbuffers/protobuf>, tag `v29.3`
(`src/google/protobuf/*.proto`), licensed under the
[BSD 3-Clause License](https://github.com/protocolbuffers/protobuf/blob/v29.3/LICENSE).

Vendored files:

- `any.proto`
- `duration.proto`
- `empty.proto`
- `field_mask.proto`
- `struct.proto`
- `timestamp.proto`
- `wrappers.proto`

The `build.rs` script adds this directory to the `protoc` include path. Only
update these files when bumping the protobuf source release; they must stay
byte-identical to upstream apart from the version noted above.
