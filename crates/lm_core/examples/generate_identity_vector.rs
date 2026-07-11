use lm_core::identity::IdentitySeed;
use lm_core::{Identity, normalize_passphrase};
use serde::Serialize;

#[derive(Serialize)]
struct IdentityVector<'a> {
    name: &'a str,
    identity_seed_hex: String,
    user_id: String,
    identity_public_key_base64: String,
    x25519_public_key_base64: String,
    storage_key_hex: String,
    passphrase_input: &'a str,
    passphrase_normalized: String,
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn main() {
    let seed = [7u8; 32];
    let identity = Identity::from_seed(IdentitySeed::from_bytes(seed)).unwrap();
    let vector = IdentityVector {
        name: "identity_v1_fixed_seed_07",
        identity_seed_hex: hex(&seed),
        user_id: identity.user_id().to_string(),
        identity_public_key_base64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            identity.identity_public_key(),
        ),
        x25519_public_key_base64: base64::Engine::encode(
            &base64::engine::general_purpose::STANDARD,
            identity.x25519_public_key(),
        ),
        storage_key_hex: hex(&identity.storage_key().unwrap()),
        passphrase_input: "  ＡＢＣ　１２３  ",
        passphrase_normalized: normalize_passphrase("  ＡＢＣ　１２３  "),
    };
    println!("{}", serde_json::to_string_pretty(&vector).unwrap());
}
