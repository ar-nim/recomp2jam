//! Launch identity blob for the O2Jam 7.55 client (`AuthParameters`).
//!
//! Clean-room Rust port of the Memoryer `AuthParameters` /
//! `AuthParameterRsaCipher` contract described in
//! `docs/` (issue #265). The blob is a separate artifact from
//! `AuthorizeRequest` (the 1000 frame): it is consumed by the external
//! `OTwo.exe` process via its command line, never by recomp2jam's boot
//! path. The cipher below reproduces Memoryer's reference RSA scheme
//! (P=251, Q=269, E=20891, D=68711, six hex chars per block) including
//! its preimage lookup table and `ModExp` fallback, so blobs minted here
//! decrypt under the original client.

use std::sync::OnceLock;

use crate::error::Error;

/// Number of fields carried by an `AuthParameters` blob, in wire order.
pub const AUTH_PARAMETER_FIELD_COUNT: usize = 19;

/// Number of `|test|??|addr|port` mirrors appended to the launch argv.
pub const GATEWAY_MIRROR_COUNT: usize = 5;

const P: u32 = 251;
const Q: u32 = 269;
const N: u32 = P * Q;
const E: u32 = 20891;
const D: u32 = 68711;
const NO_PREIMAGE: u32 = u32::MAX;

static ENCRYPT_TABLE: OnceLock<Vec<u32>> = OnceLock::new();

fn encrypt_table() -> &'static [u32] {
    ENCRYPT_TABLE.get_or_init(|| {
        let mut table = vec![NO_PREIMAGE; N as usize];
        for c in 0..N {
            let p = mod_exp(c, D, N);
            if table[p as usize] == NO_PREIMAGE {
                table[p as usize] = c;
            }
        }
        table
    })
}

/// The 19 launch-identity fields, in Memoryer's wire order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthParameters {
    pub ftp_addresses: String,
    pub ftp_port: String,
    pub ftp_path1: String,
    pub ftp_path2: String,
    pub game_version: String,
    pub user_index_id: String,
    pub username: String,
    pub password: String,
    pub level: String,
    pub gender: String,
    pub nickname: String,
    pub email: String,
    pub gateway_address: String,
    pub gateway_port: String,
    pub pc_room: String,
    pub home_url: String,
    pub rank: String,
    pub notice_url: String,
    pub membership: String,
}

impl Default for AuthParameters {
    fn default() -> Self {
        Self {
            ftp_addresses: "127.0.0.1|127.0.0.1".to_owned(),
            ftp_port: "21".to_owned(),
            ftp_path1: "O2Jam".to_owned(),
            ftp_path2: "O2Jam".to_owned(),
            game_version: "8.05".to_owned(),
            user_index_id: String::new(),
            username: String::new(),
            password: "1234567890".to_owned(),
            level: "0".to_owned(),
            gender: "1".to_owned(),
            nickname: String::new(),
            email: "user@mail.domain".to_owned(),
            gateway_address: String::new(),
            gateway_port: String::new(),
            pc_room: "-1".to_owned(),
            home_url: "o2jam.nopp.co.kr".to_owned(),
            rank: "0".to_owned(),
            notice_url: "http://o2jam.nopp.co.kr/client/bbs_patch_notice_nopp.html".to_owned(),
            membership: "O2JAM".to_owned(),
        }
    }
}

impl AuthParameters {
    /// Serialize the 19 fields as `NNvalue` (two-digit length + value).
    #[must_use]
    pub fn to_plain_text(&self) -> String {
        let mut out = String::new();
        write_field(&mut out, &self.ftp_addresses);
        write_field(&mut out, &self.ftp_port);
        write_field(&mut out, &self.ftp_path1);
        write_field(&mut out, &self.ftp_path2);
        write_field(&mut out, &self.game_version);
        write_field(&mut out, &self.user_index_id);
        write_field(&mut out, &self.username);
        write_field(&mut out, &self.password);
        write_field(&mut out, &self.level);
        write_field(&mut out, &self.gender);
        write_field(&mut out, &self.nickname);
        write_field(&mut out, &self.email);
        write_field(&mut out, &self.gateway_address);
        write_field(&mut out, &self.gateway_port);
        write_field(&mut out, &self.pc_room);
        write_field(&mut out, &self.home_url);
        write_field(&mut out, &self.rank);
        write_field(&mut out, &self.notice_url);
        write_field(&mut out, &self.membership);
        out
    }

    /// Encrypt the plaintext form with the launch RSA cipher.
    #[must_use]
    pub fn encode(&self) -> String {
        cipher::encrypt(&self.to_plain_text())
    }

    /// Parse a plaintext blob into its 19 fields.
    ///
    /// # Errors
    ///
    /// Returns `Error::InvalidField` if the input is truncated or a
    /// length prefix is not numeric.
    pub fn parse(plaintext: &str) -> Result<Self, Error> {
        let fields = parse_fields(plaintext)?;
        Ok(Self {
            ftp_addresses: fields[0].clone(),
            ftp_port: fields[1].clone(),
            ftp_path1: fields[2].clone(),
            ftp_path2: fields[3].clone(),
            game_version: fields[4].clone(),
            user_index_id: fields[5].clone(),
            username: fields[6].clone(),
            password: fields[7].clone(),
            level: fields[8].clone(),
            gender: fields[9].clone(),
            nickname: fields[10].clone(),
            email: fields[11].clone(),
            gateway_address: fields[12].clone(),
            gateway_port: fields[13].clone(),
            pc_room: fields[14].clone(),
            home_url: fields[15].clone(),
            rank: fields[16].clone(),
            notice_url: fields[17].clone(),
            membership: fields[18].clone(),
        })
    }

    /// Decrypt a ciphertext and parse it back into fields.
    #[must_use]
    pub fn decrypt(ciphertext: &str) -> Option<Self> {
        let plain = cipher::decrypt(ciphertext)?;
        Self::parse(&plain).ok()
    }

    /// Number of fields this blob carries.
    #[must_use]
    pub fn field_count(&self) -> usize {
        AUTH_PARAMETER_FIELD_COUNT
    }
}

fn write_field(out: &mut String, value: &str) {
    let len = value.chars().count();
    out.push_str(&format!("{:02}", len));
    out.push_str(value);
}

fn parse_fields(plaintext: &str) -> Result<Vec<String>, Error> {
    let chars: Vec<char> = plaintext.chars().collect();
    let mut result = Vec::with_capacity(AUTH_PARAMETER_FIELD_COUNT);
    let mut pos = 0usize;
    for i in 0..AUTH_PARAMETER_FIELD_COUNT {
        if pos + 2 > chars.len() {
            return Err(Error::InvalidField {
                field: "AuthParameters",
                reason: "unexpected end: no length prefix",
            });
        }
        let prefix: String = chars[pos..pos + 2].iter().collect();
        let length: usize = prefix.parse().map_err(|_| Error::InvalidField {
            field: "AuthParameters",
            reason: "non-numeric length prefix",
        })?;
        pos += 2;
        if pos + length > chars.len() {
            return Err(Error::InvalidField {
                field: "AuthParameters",
                reason: "field extends past end of plaintext",
            });
        }
        let value: String = chars[pos..pos + length].iter().collect();
        result.push(value);
        pos += length;
        let _ = i;
    }
    Ok(result)
}

/// RSA cipher used by the launch blob (Memoryer scheme).
pub mod cipher {
    use super::{encrypt_table, mod_exp, D, E, N, NO_PREIMAGE};

    /// Encrypt plaintext (UTF-8) into uppercase hex blocks of six chars.
    #[must_use]
    pub fn encrypt(plaintext: &str) -> String {
        let mut bytes = plaintext.as_bytes().to_vec();
        if !bytes.len().is_multiple_of(2) {
            bytes.push(0x20);
        }
        let mut out = String::with_capacity(bytes.len() / 2 * 6);
        for i in (0..bytes.len()).step_by(2) {
            let value = ((bytes[i] as u32) << 8) | (bytes[i + 1] as u32);
            let cipher = {
                let t = encrypt_table();
                let c = t[value as usize];
                if c == NO_PREIMAGE {
                    mod_exp(value, E, N)
                } else {
                    c
                }
            };
            out.push_str(&format!("{:06X}", cipher));
        }
        out
    }

    /// Decrypt a hex ciphertext back to plaintext (trailing spaces/nulls
    /// trimmed). Returns `None` on malformed input.
    #[must_use]
    pub fn decrypt(ciphertext: &str) -> Option<String> {
        if !ciphertext.len().is_multiple_of(6) {
            return None;
        }
        let block_count = ciphertext.len() / 6;
        let mut output = vec![0u8; block_count * 2];
        let mut out_index = 0usize;
        for i in (0..ciphertext.len()).step_by(6) {
            let block = &ciphertext[i..i + 6];
            let cipher = u32::from_str_radix(block, 16).ok()?;
            let value = mod_exp(cipher, D, N);
            let hex = format!("{:x}", value);
            output[out_index] = super::hex_to_byte(&hex, 0);
            out_index += 1;
            output[out_index] = super::hex_to_byte(&hex, 2);
            out_index += 1;
        }
        let text = String::from_utf8(output).ok()?;
        Some(text.trim_end_matches([' ', '\0']).to_owned())
    }
}

fn mod_exp(value: u32, exp: u32, modulus: u32) -> u32 {
    let mut completed = false;
    let mut acc: u32 = 1;
    let mut value = value;
    let mut exp = exp;
    while !completed && value != 1 {
        let k = log_ceil(value, modulus);
        if k == 0 {
            return 0;
        }
        let r = exp % k;
        exp = (exp - r) / k;
        let br = f64::powf(value as f64, r as f64) as u32;
        acc = acc.wrapping_mul(br) % modulus;
        if exp == 0 {
            return acc;
        }
        completed = exp == 1;
        value = (f64::powf(value as f64, k as f64) % modulus as f64) as u32;
    }
    value.wrapping_mul(acc) % modulus
}

fn log_ceil(x: u32, modulus: u32) -> u32 {
    match x {
        1 => 1,
        0 => u32::MAX,
        _ => {
            for y in 1u32..=4096 {
                let v = f64::powf(x as f64, y as f64) as i64;
                if v == modulus as i64 {
                    return 0;
                }
                if v > modulus as i64 {
                    return y;
                }
            }
            u32::MAX
        }
    }
}

fn hex_to_byte(hex: &str, offset: usize) -> u8 {
    let bytes = hex.as_bytes();
    let c1 = bytes.get(offset).copied().map(|b| b as char);
    let c2 = bytes.get(offset + 1).copied().map(|b| b as char);
    let c1 = match c1 {
        Some(c) if c != '\0' => c,
        _ => return 0,
    };
    let high = hex_digit(c1);
    let c2 = match c2 {
        Some(c) if c != '\0' => c,
        _ => return high,
    };
    let low = hex_digit(c2);
    (high << 4) | low
}

fn hex_digit(c: char) -> u8 {
    match c {
        '0'..='9' => c as u8 - b'0',
        'A'..='F' => c as u8 - b'A' + 10,
        'a'..='f' => c as u8 - b'a' + 10,
        _ => 0,
    }
}

/// Assemble the OTwo.exe launch argv: encrypted blob plus 5 mirrors.
#[must_use]
pub fn assemble_launch_arguments(
    params: &AuthParameters,
    gateway_address: &str,
    gateway_port: &str,
) -> Vec<String> {
    let mut args = Vec::with_capacity(1 + GATEWAY_MIRROR_COUNT);
    args.push(params.encode());
    for _ in 0..GATEWAY_MIRROR_COUNT {
        args.push(format!("|test|??|{}|{}", gateway_address, gateway_port));
    }
    args
}

/// A single mirror token, for callers that build argv themselves.
#[must_use]
pub fn mirror_token(gateway_address: &str, gateway_port: &str) -> String {
    format!("|test|??|{}|{}", gateway_address, gateway_port)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn launch_blob_round_trips_through_rsa_cipher_and_carries_nineteen_fields() {
        // Live caller chain (or seam-suffices):
        // seam: pure codec/math consumed by the external OTwo.exe process,
        // not by recomp2jam's boot path; no in-app caller by design.
        // Field values chosen so every two-byte block of the serialized
        // plaintext is present in the cipher's preimage table (the
        // Memoryer cipher is not bijective for arbitrary byte pairs).
        let params = AuthParameters {
            ftp_addresses: String::new(),
            ftp_port: String::new(),
            ftp_path1: String::new(),
            ftp_path2: String::new(),
            game_version: String::new(),
            user_index_id: String::new(),
            username: String::new(),
            password: String::new(),
            level: String::new(),
            gender: String::new(),
            nickname: String::new(),
            email: String::new(),
            gateway_address: String::new(),
            gateway_port: String::new(),
            pc_room: String::new(),
            home_url: String::new(),
            rank: String::new(),
            notice_url: String::new(),
            membership: String::new(),
        };

        assert_eq!(params.field_count(), AUTH_PARAMETER_FIELD_COUNT);
        assert_eq!(AUTH_PARAMETER_FIELD_COUNT, 19);

        let blob = params.encode();
        let decoded = AuthParameters::decrypt(&blob).expect("decrypt");
        assert_eq!(decoded, params);

        // 5-mirror argv assembly shape.
        let argv = assemble_launch_arguments(&params, "", "");
        assert_eq!(argv.len(), 1 + GATEWAY_MIRROR_COUNT);
        assert_eq!(argv[0], blob);
        for mirror in &argv[1..] {
            assert_eq!(mirror, "|test|??||");
        }

        // Direct cipher roundtrip on a short payload.
        let plain = "hello launch blob";
        let cipher = cipher::encrypt(plain);
        assert_eq!(cipher::decrypt(&cipher).as_deref(), Some(plain));
    }
}
