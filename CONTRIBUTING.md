# Contributing

IceSniff is in the foundation stage.

## License

IceSniff is released under the MIT License.

By contributing to this repository, you agree that your contributions will be licensed under the same MIT terms so the project can remain easy to adopt and extend once it is public.

## Ground rules

- Keep shared logic out of interface code.
- Prefer small explicit crates over large mixed-responsibility modules.
- Update docs alongside code changes.
- Preserve CLI and desktop parity expectations even when only one interface exists.

## Before making a feature change

1. Check `docs/architecture/overview.md`.
2. Check `docs/feature-parity-matrix.md`.
3. Record notable architectural direction in `docs/continuity-log.md`.

## Development workflow

```bash
cargo fmt
cargo test
cargo run -p icesniff-cli -- help
cargo run -p icesniff-cli -- stats path/to/capture.pcap
cargo run -p icesniff-cli -- conversations path/to/capture.pcap
```
