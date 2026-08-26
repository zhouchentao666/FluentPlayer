use sha1::{Sha1, Digest};

const PART_1_INDEXES: [usize; 8] = [23, 14, 6, 36, 16, 40, 7, 19];
const PART_2_INDEXES: [usize; 8] = [16, 1, 32, 12, 19, 27, 8, 5];
const SCRAMBLE_VALUES: [u8; 20] = [
    89, 39, 179, 150, 218, 82, 58, 252, 177, 52, 186, 123, 120, 64, 242, 133, 143, 161, 121, 179
];

fn hash_sha1(text: &str) -> String {
    let mut hasher = Sha1::new();
    hasher.update(text.as_bytes());
    let result = hasher.finalize();
    hex::encode_upper(result)
}

fn pick_hash_by_idx(hash: &str, indexes: &[usize]) -> String {
    indexes.iter().filter_map(|&idx| hash.chars().nth(idx)).collect()
}

fn base64_encode_no_special(data: &[u8]) -> String {
    use base64::Engine;
    base64::engine::general_purpose::STANDARD.encode(data)
        .replace(['/', '\\', '+', '='], "")
}

/// Generate zzcSign for QQ Music API requests
pub fn zzc_sign(text: &str) -> String {
    let hash = hash_sha1(text);
    let part1 = pick_hash_by_idx(&hash, &PART_1_INDEXES);
    let part2 = pick_hash_by_idx(&hash, &PART_2_INDEXES);

    let part3: Vec<u8> = SCRAMBLE_VALUES.iter().enumerate().map(|(i, &val)| {
        let hex_byte = u8::from_str_radix(&hash[i * 2..i * 2 + 2], 16).unwrap_or(0);
        val ^ hex_byte
    }).collect();

    let b64_part = base64_encode_no_special(&part3);

    format!("zzc{}{}{}", part1, b64_part, part2).to_lowercase()
}

/// Triple-DES S-Boxes (same as JS reference)
const SBOX: [[u8; 64]; 8] = [
    [14,4,13,1,2,15,11,8,3,10,6,12,5,9,0,7,0,15,7,4,14,2,13,1,10,6,12,11,9,5,3,8,4,1,14,8,13,6,2,11,15,12,9,7,3,10,5,0,15,12,8,2,4,9,1,7,5,11,3,14,10,0,6,13],
    [15,1,8,14,6,11,3,4,9,7,2,13,12,0,5,10,3,13,4,7,15,2,8,15,12,0,1,10,6,9,11,5,0,14,7,11,10,4,13,1,5,8,12,6,9,3,2,15,13,8,10,1,3,15,4,2,11,6,7,12,0,5,14,9],
    [10,0,9,14,6,3,15,5,1,13,12,7,11,4,2,8,13,7,0,9,3,4,6,10,2,8,5,14,12,11,15,1,13,6,4,9,8,15,3,0,11,1,2,12,5,10,14,7,1,10,13,0,6,9,8,7,4,15,14,3,11,5,2,12],
    [7,13,14,3,0,6,9,10,1,2,8,5,11,12,4,15,13,8,11,5,6,15,0,3,4,7,2,12,1,10,14,9,10,6,9,0,12,11,7,13,15,1,3,14,5,2,8,4,3,15,0,6,10,10,13,8,9,4,5,11,12,7,2,14],
    [2,12,4,1,7,10,11,6,8,5,3,15,13,0,14,9,14,11,2,12,4,7,13,1,5,0,15,10,3,9,8,6,4,2,1,11,10,13,7,8,15,9,12,5,6,3,0,14,11,8,12,7,1,14,2,13,6,15,0,9,10,4,5,3],
    [12,1,10,15,9,2,6,8,0,13,3,4,14,7,5,11,10,15,4,2,7,12,9,5,6,1,13,14,0,11,3,8,9,14,15,5,2,8,12,3,7,0,4,10,1,13,11,6,4,3,2,12,9,5,15,10,11,14,1,7,6,0,8,13],
    [4,11,2,14,15,0,8,13,3,12,9,7,5,10,6,1,13,0,11,7,4,9,1,10,14,3,5,12,2,15,8,6,1,4,11,13,12,3,7,14,10,15,6,8,0,5,9,2,6,11,13,8,1,4,10,7,9,5,0,15,14,2,3,12],
    [13,2,8,4,6,15,11,1,10,9,3,14,5,0,12,7,1,15,13,8,10,3,7,4,12,5,6,11,0,14,9,2,7,11,4,1,9,12,14,2,0,6,10,13,15,3,5,8,2,1,14,7,4,10,8,13,15,12,9,0,3,5,6,11],
];

const KEY_RND_SHIFT: [i32; 16] = [1, 1, 2, 2, 2, 2, 2, 2, 1, 2, 2, 2, 2, 2, 2, 1];
const KEY_PERM_C: [usize; 28] = [56,48,40,32,24,16,8,0,57,49,41,33,25,17,9,1,58,50,42,34,26,18,10,2,59,51,43,35];
const KEY_PERM_D: [usize; 28] = [62,54,46,38,30,22,14,6,61,53,45,37,29,21,13,5,60,52,44,36,28,20,12,4,27,19,11,3];
const KEY_COMPRESSION: [usize; 48] = [13,16,10,23,0,4,2,27,14,5,20,9,22,18,11,3,25,7,15,6,26,19,12,1,40,51,30,36,46,54,29,39,50,44,32,47,43,48,38,55,33,52,45,41,49,35,28,31];

fn bitnum(a: &[u8], b: usize, c: u32) -> u32 {
    let byte_index = (b / 32) * 4 + 3 - (b % 32) / 8;
    let bit_in_byte = 7 - (b % 8);
    let bit = (a[byte_index] >> bit_in_byte) & 1;
    (bit as u32) << c
}

fn bitnum_intr(a: u32, b: u32, c: u32) -> u32 {
    ((a >> (31 - b)) & 1) << c
}

fn bitnum_intl(a: u32, b: u32, c: u32) -> u32 {
    ((a << b) & 0x80000000) >> c
}

fn sbox_bit(a: u32) -> usize {
    ((a & 32) | ((a & 31) >> 1) | ((a & 1) << 4)) as usize
}

fn initial_permutation(input: &[u8]) -> (u32, u32) {
    let s0 =
        bitnum(input, 57, 31) | bitnum(input, 49, 30) | bitnum(input, 41, 29) | bitnum(input, 33, 28) |
        bitnum(input, 25, 27) | bitnum(input, 17, 26) | bitnum(input, 9, 25) | bitnum(input, 1, 24) |
        bitnum(input, 59, 23) | bitnum(input, 51, 22) | bitnum(input, 43, 21) | bitnum(input, 35, 20) |
        bitnum(input, 27, 19) | bitnum(input, 19, 18) | bitnum(input, 11, 17) | bitnum(input, 3, 16) |
        bitnum(input, 61, 15) | bitnum(input, 53, 14) | bitnum(input, 45, 13) | bitnum(input, 37, 12) |
        bitnum(input, 29, 11) | bitnum(input, 21, 10) | bitnum(input, 13, 9) | bitnum(input, 5, 8) |
        bitnum(input, 63, 7) | bitnum(input, 55, 6) | bitnum(input, 47, 5) | bitnum(input, 39, 4) |
        bitnum(input, 31, 3) | bitnum(input, 23, 2) | bitnum(input, 15, 1) | bitnum(input, 7, 0);

    let s1 =
        bitnum(input, 56, 31) | bitnum(input, 48, 30) | bitnum(input, 40, 29) | bitnum(input, 32, 28) |
        bitnum(input, 24, 27) | bitnum(input, 16, 26) | bitnum(input, 8, 25) | bitnum(input, 0, 24) |
        bitnum(input, 58, 23) | bitnum(input, 50, 22) | bitnum(input, 42, 21) | bitnum(input, 34, 20) |
        bitnum(input, 26, 19) | bitnum(input, 18, 18) | bitnum(input, 10, 17) | bitnum(input, 2, 16) |
        bitnum(input, 60, 15) | bitnum(input, 52, 14) | bitnum(input, 44, 13) | bitnum(input, 36, 12) |
        bitnum(input, 28, 11) | bitnum(input, 20, 10) | bitnum(input, 12, 9) | bitnum(input, 4, 8) |
        bitnum(input, 62, 7) | bitnum(input, 54, 6) | bitnum(input, 46, 5) | bitnum(input, 38, 4) |
        bitnum(input, 30, 3) | bitnum(input, 22, 2) | bitnum(input, 14, 1) | bitnum(input, 6, 0);

    (s0, s1)
}

fn inverse_permutation(s0: u32, s1: u32) -> [u8; 8] {
    let mut data = [0u8; 8];
    data[3] =
        (bitnum_intr(s1, 7, 7) | bitnum_intr(s0, 7, 6) | bitnum_intr(s1, 15, 5) | bitnum_intr(s0, 15, 4) |
         bitnum_intr(s1, 23, 3) | bitnum_intr(s0, 23, 2) | bitnum_intr(s1, 31, 1) | bitnum_intr(s0, 31, 0)) as u8;
    data[2] =
        (bitnum_intr(s1, 6, 7) | bitnum_intr(s0, 6, 6) | bitnum_intr(s1, 14, 5) | bitnum_intr(s0, 14, 4) |
         bitnum_intr(s1, 22, 3) | bitnum_intr(s0, 22, 2) | bitnum_intr(s1, 30, 1) | bitnum_intr(s0, 30, 0)) as u8;
    data[1] =
        (bitnum_intr(s1, 5, 7) | bitnum_intr(s0, 5, 6) | bitnum_intr(s1, 13, 5) | bitnum_intr(s0, 13, 4) |
         bitnum_intr(s1, 21, 3) | bitnum_intr(s0, 21, 2) | bitnum_intr(s1, 29, 1) | bitnum_intr(s0, 29, 0)) as u8;
    data[0] =
        (bitnum_intr(s1, 4, 7) | bitnum_intr(s0, 4, 6) | bitnum_intr(s1, 12, 5) | bitnum_intr(s0, 12, 4) |
         bitnum_intr(s1, 20, 3) | bitnum_intr(s0, 20, 2) | bitnum_intr(s1, 28, 1) | bitnum_intr(s0, 28, 0)) as u8;
    data[7] =
        (bitnum_intr(s1, 3, 7) | bitnum_intr(s0, 3, 6) | bitnum_intr(s1, 11, 5) | bitnum_intr(s0, 11, 4) |
         bitnum_intr(s1, 19, 3) | bitnum_intr(s0, 19, 2) | bitnum_intr(s1, 27, 1) | bitnum_intr(s0, 27, 0)) as u8;
    data[6] =
        (bitnum_intr(s1, 2, 7) | bitnum_intr(s0, 2, 6) | bitnum_intr(s1, 10, 5) | bitnum_intr(s0, 10, 4) |
         bitnum_intr(s1, 18, 3) | bitnum_intr(s0, 18, 2) | bitnum_intr(s1, 26, 1) | bitnum_intr(s0, 26, 0)) as u8;
    data[5] =
        (bitnum_intr(s1, 1, 7) | bitnum_intr(s0, 1, 6) | bitnum_intr(s1, 9, 5) | bitnum_intr(s0, 9, 4) |
         bitnum_intr(s1, 17, 3) | bitnum_intr(s0, 17, 2) | bitnum_intr(s1, 25, 1) | bitnum_intr(s0, 25, 0)) as u8;
    data[4] =
        (bitnum_intr(s1, 0, 7) | bitnum_intr(s0, 0, 6) | bitnum_intr(s1, 8, 5) | bitnum_intr(s0, 8, 4) |
         bitnum_intr(s1, 16, 3) | bitnum_intr(s0, 16, 2) | bitnum_intr(s1, 24, 1) | bitnum_intr(s0, 24, 0)) as u8;
    data
}

fn des_f(state: u32, key: &[u8]) -> u32 {
    let t1 =
        bitnum_intl(state, 31, 0) |
        ((state & 0xf0000000) >> 1) |
        bitnum_intl(state, 4, 5) |
        bitnum_intl(state, 3, 6) |
        ((state & 0x0f000000) >> 3) |
        bitnum_intl(state, 8, 11) |
        bitnum_intl(state, 7, 12) |
        ((state & 0x00f00000) >> 5) |
        bitnum_intl(state, 12, 17) |
        bitnum_intl(state, 11, 18) |
        ((state & 0x000f0000) >> 7) |
        bitnum_intl(state, 16, 23);

    let t2 =
        bitnum_intl(state, 15, 0) |
        ((state & 0x0000f000) << 15) |
        bitnum_intl(state, 20, 5) |
        bitnum_intl(state, 19, 6) |
        ((state & 0x00000f00) << 13) |
        bitnum_intl(state, 24, 11) |
        bitnum_intl(state, 23, 12) |
        ((state & 0x000000f0) << 11) |
        bitnum_intl(state, 28, 17) |
        bitnum_intl(state, 27, 18) |
        ((state & 0x0000000f) << 9) |
        bitnum_intl(state, 0, 23);

    let lrgstate: Vec<u8> = [
        (t1 >> 24) & 0xff, (t1 >> 16) & 0xff, (t1 >> 8) & 0xff,
        (t2 >> 24) & 0xff, (t2 >> 16) & 0xff, (t2 >> 8) & 0xff,
    ].iter().zip(key.iter()).map(|(&v, &k)| (v as u8) ^ k).collect();

    let new_state =
        ((SBOX[0][sbox_bit((lrgstate[0] >> 2) as u32)] as u32) << 28) |
        ((SBOX[1][sbox_bit(((lrgstate[0] & 0x03) << 4 | lrgstate[1] >> 4) as u32)] as u32) << 24) |
        ((SBOX[2][sbox_bit(((lrgstate[1] & 0x0f) << 2 | lrgstate[2] >> 6) as u32)] as u32) << 20) |
        ((SBOX[3][sbox_bit((lrgstate[2] & 0x3f) as u32)] as u32) << 16) |
        ((SBOX[4][sbox_bit((lrgstate[3] >> 2) as u32)] as u32) << 12) |
        ((SBOX[5][sbox_bit(((lrgstate[3] & 0x03) << 4 | lrgstate[4] >> 4) as u32)] as u32) << 8) |
        ((SBOX[6][sbox_bit(((lrgstate[4] & 0x0f) << 2 | lrgstate[5] >> 6) as u32)] as u32) << 4) |
        (SBOX[7][sbox_bit((lrgstate[5] & 0x3f) as u32)] as u32);

    bitnum_intl(new_state, 15, 0) | bitnum_intl(new_state, 6, 1) | bitnum_intl(new_state, 19, 2) |
    bitnum_intl(new_state, 20, 3) | bitnum_intl(new_state, 28, 4) | bitnum_intl(new_state, 11, 5) |
    bitnum_intl(new_state, 27, 6) | bitnum_intl(new_state, 16, 7) | bitnum_intl(new_state, 0, 8) |
    bitnum_intl(new_state, 14, 9) | bitnum_intl(new_state, 22, 10) | bitnum_intl(new_state, 25, 11) |
    bitnum_intl(new_state, 4, 12) | bitnum_intl(new_state, 17, 13) | bitnum_intl(new_state, 30, 14) |
    bitnum_intl(new_state, 9, 15) | bitnum_intl(new_state, 1, 16) | bitnum_intl(new_state, 7, 17) |
    bitnum_intl(new_state, 23, 18) | bitnum_intl(new_state, 13, 19) | bitnum_intl(new_state, 31, 20) |
    bitnum_intl(new_state, 26, 21) | bitnum_intl(new_state, 2, 22) | bitnum_intl(new_state, 8, 23) |
    bitnum_intl(new_state, 18, 24) | bitnum_intl(new_state, 12, 25) | bitnum_intl(new_state, 29, 26) |
    bitnum_intl(new_state, 5, 27) | bitnum_intl(new_state, 21, 28) | bitnum_intl(new_state, 10, 29) |
    bitnum_intl(new_state, 3, 30) | bitnum_intl(new_state, 24, 31)
}

type KeySchedule = [[u8; 6]; 16];

#[allow(clippy::needless_range_loop)]
fn key_schedule(key: &[u8], mode: bool) -> KeySchedule {
    let mut schedule = [[0u8; 6]; 16];
    let mut c: u32 = 0;
    let mut d: u32 = 0;
    for i in 0..28 {
        c |= bitnum(key, KEY_PERM_C[i], 31 - i as u32);
        d |= bitnum(key, KEY_PERM_D[i], 31 - i as u32);
    }
    // Ensure 28-bit by masking
    c &= 0xFFFFFFF0;
    d &= 0xFFFFFFF0;

    for i in 0..16 {
        let shift = KEY_RND_SHIFT[i] as u32;
        c = ((c << shift) | (c >> (28 - shift))) & 0xFFFFFFF0;
        d = ((d << shift) | (d >> (28 - shift))) & 0xFFFFFFF0;

        let togen = if mode { 15 - i } else { i };

        for j in 0..24 {
            schedule[togen][j / 8] |= bitnum_intr(c, KEY_COMPRESSION[j] as u32, 7 - (j % 8) as u32) as u8;
        }
        for j in 24..48 {
            schedule[togen][j / 8] |= bitnum_intr(d, KEY_COMPRESSION[j] as u32 - 27, 7 - (j % 8) as u32) as u8;
        }
    }
    schedule
}

fn des_crypt(input: &[u8], key: &KeySchedule) -> [u8; 8] {
    let (mut s0, mut s1) = initial_permutation(input);
    for k in key.iter().take(15) {
        let prev_s1 = s1;
        s1 = des_f(s1, k) ^ s0;
        s0 = prev_s1;
    }
    s0 ^= des_f(s1, &key[15]);
    inverse_permutation(s0, s1)
}

/// Decrypt QRC encrypted lyrics using Triple-DES + zlib
pub fn qrc_decrypt(encrypted_hex: &str) -> Result<String, String> {
    if encrypted_hex.is_empty() {
        return Ok(String::new());
    }

    let input = hex::decode(encrypted_hex).map_err(|e| format!("QRC hex decode error: {}", e))?;

    let key = b"!@#)(*$%123ZXC!@!@#)(NHL";
    // key_schedule mode: true = reverse order (DECRYPT), false = forward order (ENCRYPT)
    let schedule1 = key_schedule(&key[0..8], true);    // DECRYPT
    let schedule2 = key_schedule(&key[8..16], false);   // ENCRYPT
    let schedule3 = key_schedule(&key[16..24], true);   // DECRYPT

    let mut decrypted = Vec::with_capacity(input.len());
    for chunk in input.chunks(8) {
        if chunk.len() < 8 { continue; }
        let r1 = des_crypt(chunk, &schedule3);
        let r2 = des_crypt(&r1, &schedule2);
        let r3 = des_crypt(&r2, &schedule1);
        decrypted.extend_from_slice(&r3);
    }

    use flate2::read::ZlibDecoder;
    use std::io::Read;
    let mut decoder = ZlibDecoder::new(&decrypted[..]);
    let mut result = String::new();
    decoder.read_to_string(&mut result)
        .map_err(|e| format!("QRC zlib decompress error: {}", e))?;

    Ok(result)
}

/// Remove XML tags from QRC content (LyricContent="..." wrapper)
pub fn remove_tag(str: &str) -> String {
    let re1 = regex_lite::Regex::new(r#"^[\S\s]*?LyricContent=""#).unwrap();
    let re2 = regex_lite::Regex::new(r#""/>[\S\s]*$"#).unwrap();
    let result = re1.replace(str, "");
    re2.replace(&result, "").to_string()
}

fn ms_format(ms: i64) -> String {
    let total_secs = ms / 1000;
    let m = total_secs / 60;
    let s = total_secs % 60;
    let ms_rem = ms % 1000;
    format!("[{:02}:{:02}.{:03}]", m, s, ms_rem)
}

/// Parse Ceru format lyrics (QRC with word-by-word timestamps)
fn parse_ceru(lrc: &str) -> (String, String) {
    let lrc = lrc.replace('\r', "").trim().to_string();
    if lrc.is_empty() {
        return (String::new(), String::new());
    }

    let line_time_re = regex_lite::Regex::new(r"^\[(\d+),\d+\]").unwrap();
    let line_time2_re = regex_lite::Regex::new(r"^\[([\d:.]+)\]").unwrap();
    let word_time_re = regex_lite::Regex::new(r"\(\d+,\d+,\d+\)").unwrap();

    let mut lrc_lines = Vec::new();
    let mut lxlrc_lines = Vec::new();

    for line in lrc.lines() {
        let line = line.trim();

        if let Some(caps) = line_time_re.captures(line) {
            let start_ms: i64 = caps[1].parse().unwrap_or(0);
            let time_str = ms_format(start_ms);
            if time_str.is_empty() { continue; }

            let words = line_time_re.replace(line, "");
            lrc_lines.push(format!("{}{}", time_str, word_time_re.replace_all(&words, "")));

            let times_iter: Vec<_> = word_time_re.find_iter(&words).collect();
            if !times_iter.is_empty() {
                let word_arr: Vec<_> = word_time_re.split(&words).collect();
                let re = regex_lite::Regex::new(r"\((\d+),(\d+),(\d+)\)").unwrap();
                let mut lx_words = String::new();
                let mut current_start = start_ms;
                for (i, tm) in times_iter.iter().enumerate() {
                    if let Some(c) = re.captures(tm.as_str()) {
                        let duration: i64 = c[2].parse().unwrap_or(0);
                        let word_text = word_arr.get(i).unwrap_or(&"");
                        lx_words.push_str(&format!("({},{},0){}", current_start, duration, word_text));
                        current_start += duration;
                    }
                }
                lxlrc_lines.push(format!("{}{}", time_str, lx_words));
            }
        } else if line.starts_with("[offset") {
            lxlrc_lines.push(line.to_string());
            lrc_lines.push(line.to_string());
        } else if line_time2_re.is_match(line) {
            lrc_lines.push(line.to_string());
        }
    }

    (lrc_lines.join("\n"), lxlrc_lines.join("\n"))
}

fn parse_rlyric(lrc: &str) -> String {
    let lrc = lrc.replace('\r', "").trim().to_string();
    if lrc.is_empty() { return String::new(); }

    let line_time_re = regex_lite::Regex::new(r"^\[(\d+),\d+\]").unwrap();
    let word_time_re = regex_lite::Regex::new(r"\(\d+,\d+,\d+\)").unwrap();

    let mut lrc_lines = Vec::new();
    for line in lrc.lines() {
        let line = line.trim();
        if let Some(caps) = line_time_re.captures(line) {
            let start_ms: i64 = caps[1].parse().unwrap_or(0);
            let time_str = ms_format(start_ms);
            if time_str.is_empty() { continue; }
            let words = line_time_re.replace(line, "");
            lrc_lines.push(format!("{}{}", time_str, word_time_re.replace_all(&words, "")));
        }
    }
    lrc_lines.join("\n")
}

fn get_interval_ms(interval: &str) -> i64 {
    if interval.is_empty() { return 0; }
    let interval = if !interval.contains('.') {
        format!("{}.0", interval)
    } else {
        interval.to_string()
    };
    let parts: Vec<&str> = interval.split([':', '.']).collect();
    let (m, s, ms) = match parts.len() {
        3 => (parts[0].parse::<i64>().unwrap_or(0), parts[1].parse::<i64>().unwrap_or(0), parts[2].parse::<i64>().unwrap_or(0)),
        2 => (0, parts[0].parse::<i64>().unwrap_or(0), parts[1].parse::<i64>().unwrap_or(0)),
        _ => return 0,
    };
    m * 3600000 + s * 1000 + ms
}

/// Fix time tags for translated/romanized lyrics to match main lyrics
fn fix_time_tags(sub_lrc: &str, main_lrc: &str) -> String {
    let line_time2_re = regex_lite::Regex::new(r"^\[([\d:.]+)\]").unwrap();
    let sub_lines: Vec<&str> = sub_lrc.lines().collect();
    let main_lines: Vec<&str> = main_lrc.lines().collect();
    let mut result = Vec::new();
    let mut main_idx = 0;

    for sub_line in sub_lines {
        if let Some(sub_caps) = line_time2_re.captures(sub_line) {
            let words = line_time2_re.replace(sub_line, "");
            if words.trim().is_empty() { continue; }
            let t1 = get_interval_ms(&sub_caps[1]);

            while main_idx < main_lines.len() {
                if let Some(main_caps) = line_time2_re.captures(main_lines[main_idx]) {
                    let t2 = get_interval_ms(&main_caps[1]);
                    if (t1 - t2).abs() < 100 {
                        result.push(line_time2_re.replace(sub_line, &main_caps[0]).to_string());
                        main_idx += 1;
                        break;
                    }
                }
                main_idx += 1;
            }
        }
    }
    result.join("\n")
}

/// Parse QRC lyrics: lrc (main), tlrc (translated), rlrc (romanized)
pub fn parse_qrc_lyrics(lrc: &str, tlrc: &str, rlrc: &str) -> serde_json::Value {
    let mut lyric = String::new();
    let mut crlyric = String::new();
    let mut rlyric = String::new();
    let mut tlyric = String::new();

    if !lrc.is_empty() {
        let clean_lrc = remove_tag(lrc);
        let (parsed_lyric, _parsed_lxlyric) = parse_ceru(&clean_lrc);
        lyric = parsed_lyric;
        // Store raw decrypted QRC (same as JS: info.crlyric = lrc)
        // Frontend parseQrc() expects original QRC format with [ms,dur] and (off,dur,0) tags
        crlyric = lrc.to_string();
    }
    if !rlrc.is_empty() {
        let clean_rlrc = remove_tag(rlrc);
        let parsed_rlyric = parse_rlyric(&clean_rlrc);
        rlyric = fix_time_tags(&parsed_rlyric, &lyric);
    }
    if !tlrc.is_empty() {
        let clean_tlrc = remove_tag(tlrc);
        tlyric = fix_time_tags(&clean_tlrc, &lyric);
    }

    serde_json::json!({
        "lyric": lyric,
        "tlyric": tlyric,
        "rlyric": rlyric,
        "crlyric": crlyric,
        "source": "tx"
    })
}
