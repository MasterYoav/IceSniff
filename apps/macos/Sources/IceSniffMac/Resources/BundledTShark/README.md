Bundled tshark runtime assets are staged here by `scripts/sync-bundled-tshark.sh`.

The release build copies a full `Wireshark.app` into this directory so IceSniff can
use the bundled `tshark` executable without requiring a separate end-user install.
