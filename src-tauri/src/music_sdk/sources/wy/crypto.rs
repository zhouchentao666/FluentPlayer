use aes::Aes128;
use aes::cipher::{block_padding::Pkcs7, BlockEncryptMut, KeyInit, KeyIvInit};
use cbc::Encryptor as CbcEncryptor;

const IV: [u8; 16] = [0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
                      0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08];
const PRESET_KEY: [u8; 16] = *b"0CoJUm6Qyw8W8jud";
const LINUXAPI_KEY: [u8; 16] = *b"rFgB&h#%2?^eDg:Q";
const EAPI_KEY: [u8; 16] = *b"e82ckenh8dichen8";

const RSA_PUBLIC_KEY: &str = "-----BEGIN PUBLIC KEY-----\n\
MIGfMA0GCSqGSIb3DQEBAQUAA4GNADCBiQKBgQDgtQn2JZ34ZC28NWYpAUd98iZ3\n\
7BUrX/aKzmFbt7clFSs6sXqHauqKWqdtLkF2KexO40H1YTX8z2lSgBBOAxLsvakl\n\
V8k4cBFK9snQXE9/DDaFt6Rr7iVZMldczhC0JNgTz+SHXT6CBHuX3e9SdB1Ua44o\n\
ncaTWz7OBGLbCiK45wIDAQAB\n\
-----END PUBLIC KEY-----";

const BASE62: &[u8] = b"abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";

fn aes_cbc_encrypt(data: &[u8], key: &[u8; 16]) -> Vec<u8> {
    let cipher = CbcEncryptor::<Aes128>::new(key.into(), &IV.into());
    let mut buf = vec![0u8; data.len() + 16];
    buf[..data.len()].copy_from_slice(data);
    cipher.encrypt_padded_mut::<Pkcs7>(&mut buf, data.len()).unwrap().to_vec()
}

fn aes_ecb_encrypt(data: &[u8], key: &[u8; 16]) -> Vec<u8> {
    let cipher = Aes128::new(key.into());
    let mut buf = vec![0u8; data.len() + 16];
    buf[..data.len()].copy_from_slice(data);
    cipher.encrypt_padded_mut::<Pkcs7>(&mut buf, data.len()).unwrap().to_vec()
}

/// Weapi encryption: double AES-128-CBC + RSA
/// Returns (params, encSecKey) form data
pub fn weapi_encrypt(object: &serde_json::Value) -> (String, String) {
    use rand::Rng;
    let text = serde_json::to_string(object).unwrap_or_default();

    // Generate random 16-byte secret key (base62 chars)
    let mut rng = rand::thread_rng();
    let secret_key: [u8; 16] = std::array::from_fn(|_| BASE62[rng.gen_range(0..62)]);

    // First AES-CBC with preset key
    let first_encrypted = aes_cbc_encrypt(text.as_bytes(), &PRESET_KEY);
    let first_b64 = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &first_encrypted);

    // Second AES-CBC with secret key
    let second_encrypted = aes_cbc_encrypt(first_b64.as_bytes(), &secret_key);
    let params = base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &second_encrypted);

    // RSA encrypt the reversed secret key
    let mut reversed_key = secret_key;
    reversed_key.reverse();
    let enc_sec_key = rsa_encrypt_raw(&reversed_key);

    (params, enc_sec_key)
}

fn rsa_encrypt_raw(data: &[u8]) -> String {
    use rsa::{RsaPublicKey, traits::PublicKeyParts};
    use rsa::pkcs8::DecodePublicKey;

    // Pad to 128 bytes (RSA_NO_PADDING behavior)
    let mut padded = vec![0u8; 128 - data.len()];
    padded.extend_from_slice(data);

    let public_key = RsaPublicKey::from_public_key_pem(RSA_PUBLIC_KEY).unwrap();
    // Raw RSA (no padding) - compute m^e mod n directly

    // Compute m^e mod n manually
    let n = public_key.n();
    let e = public_key.e();
    let m = rsa::BigUint::from_bytes_be(&padded);
    let result = m.modpow(e, n);
    let mut result_bytes = result.to_bytes_be();
    // Ensure 128 bytes output
    while result_bytes.len() < 128 {
        result_bytes.insert(0, 0);
    }
    hex::encode(&result_bytes)
}

/// Eapi encryption: AES-128-ECB with MD5 digest
pub fn eapi_encrypt(url: &str, data: &str) -> String {
    use md5::{Digest, Md5};
    let message = format!("nobody{}use{}md5forencrypt", url, data);
    let digest = Md5::digest(message.as_bytes());
    let digest_hex = hex::encode(digest);
    let plaintext = format!("{}-36cd479b6b5-{}-36cd479b6b5-{}", url, data, digest_hex);
    let encrypted = aes_ecb_encrypt(plaintext.as_bytes(), &EAPI_KEY);
    hex::encode(&encrypted).to_uppercase()
}

/// Linuxapi encryption: AES-128-ECB
pub fn linuxapi_encrypt(payload: &str) -> String {
    let encrypted = aes_ecb_encrypt(payload.as_bytes(), &LINUXAPI_KEY);
    hex::encode(&encrypted).to_uppercase()
}

/// Build weapi form body
pub fn weapi_form(object: &serde_json::Value) -> String {
    let (params, enc_sec_key) = weapi_encrypt(object);
    format!("params={}&encSecKey={}", urlencoding::encode(&params), urlencoding::encode(&enc_sec_key))
}

/// Build eapi form body
pub fn eapi_form(url: &str, data: &serde_json::Value) -> String {
    let data_str = serde_json::to_string(data).unwrap_or_default();
    let eparams = eapi_encrypt(url, &data_str);
    format!("params={}", eparams)
}

/// Build linuxapi form body
pub fn linuxapi_form(payload: &serde_json::Value) -> String {
    let payload_str = serde_json::to_string(payload).unwrap_or_default();
    let eparams = linuxapi_encrypt(&payload_str);
    format!("eparams={}", eparams)
}
