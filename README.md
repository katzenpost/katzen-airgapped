# katzen-airgapped

An experimental Rust client for researchers studying a split cold/hot workflow for [Katzenpost](https://github.com/katzenpost): an offline "cold" machine composes and encrypts messages and shows them as QR frames, and an online "hot" machine scans the frames and hands them to `kpclientd`.

This is a research prototype, not a messaging client. It exercises QR framing and reassembly, an encrypted local mailbox and a kernel network sandbox. The cold side builds packets in a custom 1024-byte format, not Katzenpost Sphinx; `kpclientd` carries such a packet to the courier service as an opaque payload, which the courier does not understand. To send messages, use a Katzenpost client such as [katzenqt](https://github.com/katzenpost/katzenqt). The open problems are listed in [docs/REVIEW_2026-10-30.md](docs/REVIEW_2026-10-30.md).

![katzen GUI](katzen-airgapped-gui.png)

## Quick start

Requires a stable Rust toolchain; the Slint GUI needs the usual Linux desktop libraries (fontconfig, X11 or Wayland).

```bash
cargo build --locked
export KATZEN_HOME=$(mktemp -d)   # keep test state out of the repository
target/debug/katzen status
target/debug/katzen-cold send --message test --raw --loops 1 \
  | grep -o 'kpag://.*' \
  | target/debug/katzen-hot ingest --spool-dir "$KATZEN_HOME/hot"
```

The last command runs both sides on one machine: `katzen-cold` prints the QR frames as text and `katzen-hot` reassembles them. If a `kpclientd` is reachable (it also tries the abstract socket `@katzenpost`), the packet is sent and a receipt is written to `$KATZEN_HOME/hot/sent/`. Otherwise the packet is saved to `$KATZEN_HOME/hot/outbox/`; prefix the `katzen-hot` command with `unshare -rn` to force this offline path. `target/debug/katzen gui` opens the desktop interface.

## Binaries and modules

| Path | Purpose |
|---|---|
| [`src/main.rs`](src/main.rs) | `katzen`: unified CLI (`vault`, `spooler`, `receive`, `gui`, `status`) |
| [`src/cold_main.rs`](src/cold_main.rs) | `katzen-cold`: offline side; builds packets and shows QR frames |
| [`src/hot_main.rs`](src/hot_main.rs) | `katzen-hot`: online side; reads QR frames and forwards or spools them |
| [`src/lib.rs`](src/lib.rs) | `KPAG` QR chunk framing and reassembly |
| [`src/pigeonhole.rs`](src/pigeonhole.rs) | Encrypted mailbox (`KAV1`) and demonstration box envelope (`KPB2`) |
| [`src/sphinx.rs`](src/sphinx.rs) | Custom packet format and PKI consensus decoding |
| [`src/thinclient.rs`](src/thinclient.rs) | CBOR codec and `kpclientd` thin-client connection |
| [`src/sandbox.rs`](src/sandbox.rs) | Network isolation with Linux namespaces (`--sandbox`) |
| [`src/ui.rs`](src/ui.rs), [`src/virtual_keyboard.rs`](src/virtual_keyboard.rs), [`src/identicon.rs`](src/identicon.rs) | Slint GUI, on-screen keyboard, key identicons |
| [`config/client.toml`](config/client.toml) | `kpclientd` configuration used when a bundled daemon is present |

Storage, envelope, cold-to-hot path and sandbox details are in [docs/DESIGN.md](docs/DESIGN.md).

## Status and security boundary

Version 0.1.0. The mailbox is encrypted with XChaCha20-Poly1305 under an Argon2id key from a passphrase; there is no recovery. The `KPB2` envelope's BLAKE3 tag is not a verifiable signature. PKI documents are decoded without signature verification. `--sandbox` fails if the kernel cannot isolate the network; it does not prove an air gap. Treat every QR or removable-media import as untrusted input. No two-device test and no end-to-end send and reply test has passed. The test gates are in [docs/SECURITY_TEST_PLAN.md](docs/SECURITY_TEST_PLAN.md).

## Testing and contributing

```bash
cargo test --locked
```

Unit tests cover local invariants only. Issues and pull requests are welcome on GitHub. The repository has no licence file yet.
