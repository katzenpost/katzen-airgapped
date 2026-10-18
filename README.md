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

## 2. Command Line Interface

### Check Environment & Sandbox Status
```bash
katzen status
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
* **Integrity**: BLAKE3 checksums guarding every `KPAG` frame.
* **Memory Safety**: 100% pure Rust, zero-cost abstractions, zero external C daemon dependencies.

---

## 4. Author & License

- **Author**: Gabriel Virebent (`gabrix73`)
- **Reference**: [katzenpost/katzenpost#1238](https://github.com/katzenpost/katzenpost/issues/1238)
- **License**: AGPL-3.0 / Apache-2.0
