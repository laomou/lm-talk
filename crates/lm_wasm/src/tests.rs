//! Private unit tests for implementation details that are not part of the WASM API.

use super::*;
use serde_json::Value;

#[test]
fn wasm_identity_from_seed_matches_native_user_id() {
    let seed = [42u8; lm_core::identity::IDENTITY_SEED_LEN];
    let wasm_identity = wasm_identity_from_seed(seed).unwrap();
    let native_identity = Identity::from_seed(IdentitySeed::from_bytes(seed)).unwrap();
    assert_eq!(wasm_identity.user_id(), native_identity.user_id());
    assert_eq!(
        wasm_identity.identity_public_key(),
        native_identity.identity_public_key()
    );
    assert_eq!(
        wasm_identity.x25519_public_key(),
        native_identity.x25519_public_key()
    );
}

#[test]
fn wasm_identity_matches_native_test_vector() {
    let vector: Value =
        serde_json::from_str(include_str!("../../../test-vectors/identity_v1.json")).unwrap();
    let wasm_identity =
        wasm_identity_from_seed([7u8; lm_core::identity::IDENTITY_SEED_LEN]).unwrap();
    assert_eq!(wasm_identity.user_id().to_string(), vector["user_id"]);
    assert_eq!(
        BASE64.encode(wasm_identity.identity_public_key()),
        vector["identity_public_key_base64"]
    );
    assert_eq!(
        BASE64.encode(wasm_identity.x25519_public_key()),
        vector["x25519_public_key_base64"]
    );
}
