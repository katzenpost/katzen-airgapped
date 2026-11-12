# katzen-airgapped design notes

These notes were moved out of the README. The open findings and
release gates are in [REVIEW_2026-10-30.md](REVIEW_2026-10-30.md); the
test gates are in [SECURITY_TEST_PLAN.md](SECURITY_TEST_PLAN.md).

## Local storage

- The local mailbox is encrypted with XChaCha20-Poly1305 (`KAV1`). A
  passphrase derives the key with Argon2id (64 MiB, three passes). The
  passphrase is requested once per process. Keep a separate, protected
  backup and retain the passphrase: there is no recovery mechanism.
- The mailbox lives under `$KATZEN_HOME/vault_data/`. Without
  `KATZEN_HOME`, the root is derived from the executable's location
  (for example the repository root when run from `target/debug/`).
- The first successful unlock of a legacy JSON mailbox migrates it to
  `KAV1` using an atomic replacement. A failed unlock leaves the legacy
  file intact. Old backups, filesystem snapshots, swap and existing
  `spool/inbox/*.inbound` files may still contain plaintext; this
  program does not remove them.

## Demonstration envelope

The demonstration box envelope (`KPB2`) uses ephemeral X25519 and
XChaCha20-Poly1305, authenticating the envelope version, ephemeral
public key, recipient ID, nonce and ciphertext. Old unauthenticated
envelopes are rejected. The BLAKE3 tag is not a verifiable asymmetric
signature and is not a BACAP wire signature.

## Cold to hot path

1. `katzen-cold` (or `katzen vault`) builds a packet in the custom
   1024-byte format in `src/sphinx.rs` and splits it into `KPAG` QR
   chunks (`src/lib.rs`).
2. `katzen-hot ingest` reads one QR string per line from stdin;
   `katzen-hot scan` reads them from a webcam through `zbarcam`.
3. The hot side reassembles the chunks and connects to `kpclientd`
   over a Unix socket (default `/run/katzenpost/client.sock`, then the
   abstract socket `@katzenpost`) or a loopback-only TCP address. On a
   network-isolated host it will not start a bundled `kpclientd`. It sends the payload as a thin-client
   CBOR request. If a `MessageSentEvent` comes back, a receipt is
   written to `<spool>/sent/receipt_<seq>.json`.
4. If the daemon is unreachable, the packet is saved with mode `0600`
   to `<spool>/outbox/` for later dispatch.

`kpclientd` carries the packet through the mixnet to the courier
service as an opaque payload. Because the packet is not a Pigeonhole
request, the courier does not understand it; a receipt shows only
that `kpclientd` sent it.

## PKI consensus

`ConsensusTopology::from_pki_document` decodes a CBOR PKI document and
extracts the epoch, mix nodes, layers (1 to 3), addresses and Sphinx
public keys. It does not verify the document's signatures.

## Sandbox

`--sandbox` calls `unshare` with new user and network namespaces and
fails if the kernel refuses. A failed connectivity probe is not
evidence of an air gap; physical isolation must be established by the
operator and checked independently.

## Two-device testing

The QR path can be tested on one machine with synthetic data. A
two-device test should use a dedicated offline laptop for the cold
side, with radios and Ethernet disabled, and an online machine for the
hot side. Android airplane mode is useful for interface experiments
but is not the reference air-gap configuration, because Wi-Fi and
Bluetooth can remain enabled.

## GUI

`katzen gui` opens a Slint desktop interface with in-window passphrase
unlock and vault initialisation, an on-screen keyboard and inline
error reporting.
