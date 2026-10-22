# Security

MCQ Advanced is open source, so nothing it protects may depend on hidden code. Protection comes
from keys alone (Kerckhoffs's principle), built from standard primitives with no home-made
cryptography. This page says what is guaranteed, what is not, and how it works.

**Status:** the cryptographic core and the command line are implemented and tested; the player
and the editor that will use them are not released yet. Drawing and shuffling per student come
with the player.

## Reporting a vulnerability

Use GitHub's private vulnerability reporting on this repository ("Security" tab, "Report a
vulnerability"). Please do not open a public issue for a security problem.

## What is guaranteed, and what is not

During an exam the paper is necessarily readable on the student's device: no software can stop a
photograph or a memory dump. The guarantees below hold within that limit.

| Threat | What MCQ Advanced does |
|---|---|
| Reading the paper before the exam | The paper is encrypted; it opens with a code the teacher gives when the exam starts. |
| Reading the answers during the exam | The answer key is encrypted apart, with a key students do not have. Correct flags, explanations and rules are absent from the student copy; only the number of options to tick is announced. |
| Circulating a forged paper | Every package is signed by the teacher. The player shows the teacher's fingerprint, also printed on the room sheet: a forged package shows another one. |
| Altering a package or a copy | Any changed byte makes the signature or the authenticated encryption fail; the file is refused. |
| Reading or forging another student's copy | A copy is sealed for the teacher's key alone. The student's name is declared, as on paper; the receipt code shown at submission ties a copy to the student who received it. |
| Copying from a neighbour | Questions and options can be drawn and shuffled per student. |
| Changing answers after seeing the correction | The copy is sealed before any correction is shown; the correction opens with an end code the teacher reveals once every copy is in. |
| Cheating on time | Elapsed time and focus losses are recorded in the copy, **not guaranteed**: without a server, a device's clock can be changed. |
| Searching the web during the exam | Focus losses are **detected and reported, not prevented**. |

Practice packages have no opening code: their answers are readable at once, and the player says so.

## Keys and codes

- **Teacher identity**, created once: an Ed25519 key signs packages, an X25519 key receives
  copies and answer keys. The identity file is encrypted with the teacher's passphrase
  (Argon2id, at least 12 characters). **Keep a backup**: without the identity file and its
  passphrase, copies cannot be read.
- **Opening code**, generated with each package: 12 symbols of Crockford base 32, such as
  `K7QM-2XWD-9FHA`, 60 random bits. Typing is forgiving: case, dashes and the letters O, I and L
  (read as 0, 1 and 1) do not matter. A teacher can choose a phrase instead (at least 12
  characters), at the cost of a guessable secret if the phrase is weak.
- **End code**, optional, same form: opens the answer key once the teacher reveals it.
- **Receipt code**, shown to the student when a copy is sealed: 50 bits of the copy's SHA-256.

## Primitives

All from the RustCrypto and dalek projects, pure Rust, also built for WebAssembly:

| Purpose | Primitive | Crate |
|---|---|---|
| Key from a code or passphrase | Argon2id v1.3; 19 MiB, 2 passes for codes, 64 MiB, 3 passes for passphrases; settings stored in each file | `argon2` |
| Encryption of the paper, answer key and identity | XChaCha20-Poly1305, random 192-bit nonces | `chacha20poly1305` |
| Signature of packages | Ed25519, strict verification | `ed25519-dalek` |
| Sealing for the teacher | HPKE (RFC 9180), base mode, DHKEM X25519 with HKDF-SHA256, ChaCha20-Poly1305 | `hpke` |
| Hashes | SHA-256 | `sha2` |

Secrets in memory are wiped after use (`zeroize`). Every decoder bounds its reads by the data
present and refuses malformed input without panicking.

## Files

Integers are little-endian; `bytes` fields carry a `u32` length.

**Identity (`.mcqid`)**: `"MCQI"`, version, the two public keys, Argon2id settings, salt, then
the encrypted secret keys; every byte before them is authenticated.

**Package (`.mcqpack`)**: a 130-byte header (`"MCQP"`, version, flags, paper id, the teacher's
public keys, Argon2id settings, two salts), then:

1. the content key, encrypted under the opening code (in clear for practice packages);
2. the student copy and its media, encrypted under the content key;
3. the answer key's key, sealed with HPKE for the teacher;
4. the same key encrypted under the end code, when there is one;
5. the same key in clear, for practice packages only;
6. the full paper, encrypted under the answer key's key;
7. an Ed25519 signature over everything before it.

Each encrypted field is bound to the header (SHA-256 of the header plus a label naming the
field, as associated data), so fields cannot be moved between packages.

**Copy (`.mcqres`)**: `"MCQR"`, version, paper id, the teacher's sealing key, then the HPKE
encapsulated key and ciphertext of the answers, with the header as associated data.

The exact layouts are documented in `crates/mcq-crypto/src`. Reproducible test vectors, from a
fixed seed, are in `crates/mcq-crypto/tests/vectors.txt`; the tests also alter every byte of
each file and check that it is refused.

## For teachers

- Print the fingerprint on the room sheet, and ask students to compare it with the one their
  player shows.
- Give the opening code when the exam starts, not before.
- Reveal the end code only once every copy has been handed in.
- Keep the identity file and its passphrase apart, with a backup of the file.
