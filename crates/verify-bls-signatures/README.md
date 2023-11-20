BLS signature utility crate
=============================

This is a simple Rust crate which can be used to create and verify BLS signatures
over the BLS12-381 curve. This follows the
[IETF draft for BLS signatures](https://datatracker.ietf.org/doc/draft-irtf-cfrg-bls-signature/),
using the "short signature" variation, where signatures are in G1 and
public keys are in G2.

For historical reasons, this crate is named `ic-verify-bls-signature`,
but it also supports signature generation.
