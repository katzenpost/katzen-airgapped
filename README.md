# katzen-airgapped
## The Schrödinger State of Anonymous Mixnet Messaging

> *"In the sealed box: neither dead nor alive, but in an unobserved superposition of both.*  
> *In the mixnet: neither real communication nor dummy cover noise, but mathematically indistinguishable until the final hop.*  
> *In the air-gapped vault: identity and keys immune to quantum collapse under adversarial observation."*

---

`katzen-airgapped` is a sovereign, zero-dependency, universal client for the **Katzenpost** post-quantum mix network. It implements a strict split-trust architecture between an isolated cryptographic engine (**The Vault**) and a blind, untrusted network transport (**The Spooler**).

```text
    /\_/\   
   ( o.o )   KATZENPOST AIR-GAPPED: SCHRÖDINGER MIXNET CLIENT
    > ^ <    Superposition: [Vault | Spooler] • Post-Quantum Resilient
```

---

## 1. Core Architectural Pillars

1. **The Quantum Duality (Single Binary)**:
   The single compiled executable `katzen` exists in a dual state:
   - **`katzen vault` (Offline Vault)**: Holds private keys, composes messages, computes multi-hop Sphinx onion packets and generates SURBs. Zero network connectivity.
   - **`katzen spooler` (Blind Relay)**: Interfaces with the Katzenpost mixnet (over QUIC/UDP or local daemon), emits Poisson cover traffic, and retrieves opaque Pigeonhole mail. Zero access to keys or plaintexts.
   - **`katzen standalone`**: Collapses both states into a single unified in-memory process on connected workstations, with optional kernel-enforced sandboxing (`--sandbox`).

2. **Air-Gap Bridges (Zero-Contact Optical & Removable Media)**:
   - **Optical Data Diode**: High-density terminal Unicode half-block QR carousels (`kpag://` framing) captured via webcam or mobile camera.
   - **Removable Media Spool (USB / SD Card)**: File-based queue (`outbox/` and `inbox/`) using strictly bounded 1024-byte Sphinx binary payloads.

3. **Kernel-Enforced Logical Air-Gap**:
   When running on a single connected host, `katzen` can invoke OS-level isolation:
   - **Linux**: Network Namespaces (`CLONE_NEWUSER | CLONE_NEWNET`) and `PR_SET_NO_NEW_PRIVS`.
   - **OpenBSD**: `pledge("stdio rpath wpath cpath")` omitting the `inet` promise.

---

### Check Environment, Sandbox & Secrecy Score Model
```bash
katzen status
```

### Sovereign GUI (Native Slint Desktop Interface)
Launch the graphical interface with built-in Schrödinger quantum state monitor and optional grouped virtual keyboard:
```bash
katzen gui
```
*(If run on a system without an active X11 or Wayland display, `katzen` automatically transitions to the interactive terminal TUI fallback).*

### Anti-Keylogger Virtual Keyboard & Secrecy Score Model
The virtual keyboard is **optional**, designed to defend high-risk workstations against hardware USB keyloggers, screen/coordinate sniffers, and acoustic cryptanalysis:
- **Partitioned Layout**: Keys are segregated strictly into distinct groups (**Numbers**, **Letters**, and **Symbols**).
- **Intra-Group Shuffling**: Digits scramble only among digits; letters scramble only among letters; symbols scramble only among symbols. This preserves human ergonomic scanning while randomizing click/pointer coordinates.
- **Dynamic Secrecy Score**:
  - **Standard Mode (Physical Keyboard)**: `3/5 ⭐⭐⭐ (70%)` — Kernel namespace or physical air-gap active.
  - **Quantum Shielding (Virtual Keyboard)**: `5/5 ⭐⭐⭐⭐⭐ (100%)` — Full isolation from keyboard sniffing vectors.
  - **Calm, Non-Alarmist UI**: Zero panic banners or flashing red lights; clear, actionable OPSEC guidance only.

### Sovereign Pigeonhole Mailbox (Katzenpost BACAP)
Manage asynchronous mailboxes offline:
```bash
# Display your local sovereign Box ID identity:
katzen vault mailbox identity

# Compose a message draft (with optional anti-keylogger virtual keyboard):
katzen vault mailbox deposit --recipient <32-byte-hex-box-id> --virtual-keyboard

# List stored inbound and outbound messages:
katzen vault mailbox list

# Generate an anonymous SURB poll query for the Pigeonhole mailbox service:
katzen vault mailbox poll

# Ingest and decrypt an inbound reply envelope:
katzen vault mailbox ingest --file /media/usb/spool/inbox/reply.inbound
```

### Standalone Everyday Mode
Run all-in-one unified mode on your workstation:
```bash
katzen standalone --message "Hello from Katzenpost" --sandbox
```

### Air-Gapped Physical Separation

#### Step A: Offline Vault (Cold Terminal)
Compose a 3-hop Sphinx packet and save to a removable media spool:
```bash
katzen vault send --message "Confidential message" --spool-dir /media/usb/spool
```
Or project an animated high-density terminal QR carousel for camera capture:
```bash
katzen vault send --message "Confidential message"
```

#### Step B: Online Spooler (Hot Relay)
Monitor the removable media spool and transmit packets to the Katzenpost mixnet:
```bash
katzen spooler run --spool-dir /media/usb/spool
```
Or scan an animated QR carousel directly using a webcam:
```bash
katzen spooler scan --device /dev/video0
```

#### Step C: Offline Vault Reply Decryption
Decrypt received response payloads deposited in the inbound spool:
```bash
katzen vault receive --spool-dir /media/usb/spool
```

---

## 3. Cryptographic Invariants

* **Packet Geometry**: Fixed-size 1024-byte Sphinx onion packets (ChaCha20, BLAKE3 keyed-MAC, X25519/PQ ML-KEM).
* **Return Path**: Single-Use Reply Blocks (SURBs) ensuring blind, uncorrelated responses.
* **Asynchronous Mailbox**: Katzenpost BACAP (Box Access Capability) with 32-byte Box IDs and 64-byte BLAKE3 signatures.
* **Integrity**: BLAKE3 checksums guarding every `KPAG` frame.
* **Memory Safety**: 100% pure Rust, zero-cost abstractions, zero external C daemon dependencies.

---

## 4. Author & License

- **Author**: Gabriel Virebent (`gabrix73`)
- **Reference**: [katzenpost/katzenpost#1238](https://github.com/katzenpost/katzenpost/issues/1238)
- **License**: AGPL-3.0 / Apache-2.0
