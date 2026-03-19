IceSniff bundles the `tshark` packet analysis runtime from Wireshark as a separate
third-party executable for extended protocol coverage.

Compliance notes:
- Wireshark and tshark are distributed under GPL-2.0.
- IceSniff invokes the bundled tshark executable as a separate process.
- The release process must include the corresponding Wireshark source archive for the
  exact bundled build via `ICESNIFF_WIRESHARK_SOURCE_ARCHIVE`.
- Do not remove or restrict access to these notices or the corresponding source bundle.
