# IceSniff macOS 1.1.0

## Highlights

- Added bundled `tshark` / Wireshark runtime support for broader protocol coverage.
- Improved packet protocol labeling so packet lists align much more closely with Wireshark output.
- Added dynamic AI model availability based on configured API keys and installed local CLIs.
- Added an `Offline Assistant` fallback so AI help still works without provider setup.
- Hardened AI credential handling with device-local Keychain storage, ephemeral requests, disabled caching, and safer error surfacing.
- Documented AI credential and privacy behavior in the app and repository docs.

## Included In This Release

- Native macOS app bundle
- Bundled Rust CLI and capture helper
- Bundled `tshark` runtime
- GPL compliance materials for bundled Wireshark components

## Notes

- Hosted AI providers only receive packet content when you explicitly send a chat request through that provider.
- The bundled Wireshark runtime is shipped as a local dependency and is accompanied by third-party notices and source-archive compliance materials.
