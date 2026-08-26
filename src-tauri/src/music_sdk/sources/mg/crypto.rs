const DELTA: i64 = 2654435769;
const MIN_LENGTH: usize = 32;

const KEY: [i64; 9] = [
    27303562373562475,
    18014862372307051,
    22799692160172081,
    34058940340699235,
    30962724186095721,
    27303523720101991,
    27303523720101998,
    31244139033526382,
    28992395054481524,
];

fn to_i64(n: i128) -> i64 {
    const MAX: i128 = i64::MAX as i128;
    const MIN: i128 = i64::MIN as i128;
    if n > MAX { to_i64(n - (1i128 << 64)) }
    else if n < MIN { to_i64(n + (1i128 << 64)) }
    else { n as i64 }
}

fn hex_to_i64(s: &str) -> i64 {
    i64::from_str_radix(s, 16).unwrap_or(0)
}

fn i64_to_bytes(v: i64) -> [u8; 8] {
    v.to_le_bytes()
}

fn tea_decrypt(data: &mut [i64]) {
    let length = data.len() as i64;
    if length < 1 { return; }

    let mut v0 = data[0];
    let mut sum = to_i64((6 + 52 / length as i128) * DELTA as i128);

    while sum != 0 {
        let e = to_i64((sum as i128 >> 2) & 3);
        let mut i = length - 1;

        while i > 0 {
            let prev = data[(i - 1) as usize];
            let idx = (i & 3) as usize ^ (e as usize & 3);
            let k = KEY[idx];
            data[i as usize] = to_i64(
                data[i as usize] as i128 - (
                    ((v0 as i128 ^ sum as i128) + (prev as i128 ^ k as i128))
                    ^ (((prev as i128 >> 5) ^ ((v0 as i128) << 2))
                    + ((v0 as i128 >> 3) ^ ((prev as i128) << 4)))
                )
            );
            v0 = data[i as usize];
            i -= 1;
        }

        let last = data[(length - 1) as usize];
        let idx = (i & 3) as usize ^ (e as usize & 3);
        let k = KEY[idx];
        data[0] = to_i64(
            data[0] as i128 - (
                ((k as i128 ^ last as i128) + (v0 as i128 ^ sum as i128))
                ^ (((last as i128 >> 5) ^ ((v0 as i128) << 2))
                + ((v0 as i128 >> 3) ^ ((last as i128) << 4)))
            )
        );
        v0 = data[0];
        sum = to_i64(sum as i128 - DELTA as i128);
    }
}

fn to_bigint_array(data: &str) -> Vec<i64> {
    let len = data.len() / 16;
    (0..len).map(|i| hex_to_i64(&data[i * 16..(i + 1) * 16])).collect()
}

fn long_arr_to_string(data: &[i64]) -> String {
    data.iter()
        .flat_map(|v| i64_to_bytes(*v))
        .collect::<Vec<u8>>()
        .chunks(2)
        .filter_map(|pair| {
            if pair.len() == 2 {
                let code = u16::from_le_bytes([pair[0], pair[1]]);
                char::from_u32(code as u32)
            } else {
                None
            }
        })
        .collect()
}

/// Decrypt MRC (Migu encrypted lyric) using TEA cipher
pub fn decrypt_mrc(data: &str) -> String {
    if data.len() < MIN_LENGTH { return data.to_string(); }
    let mut arr = to_bigint_array(data);
    tea_decrypt(&mut arr);
    long_arr_to_string(&arr)
}
