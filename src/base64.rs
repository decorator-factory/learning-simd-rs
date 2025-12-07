// based on https://mcyoung.xyz/2023/11/27/simd-base64
// the blogpost had some annoying typos. smh my head
// https://github.com/mcy/vb64 (Apache 2.0 License)

use core::mem::MaybeUninit;
use core::mem::transmute;
use core::simd::prelude::*;
use core::simd::{
    LaneCount,
    SupportedLaneCount,
};

use crate::utils::unroll;

#[derive(Debug, Clone, Copy)]
#[allow(dead_code)]
pub struct B64Error;

/// Function yoinked from blogpost
pub fn unb64_nonsimd(data: &[u8], out: &mut Vec<u8>) -> Result<(), B64Error> {
    let data = match data {
        [p @ .., b'=', b'='] | [p @ .., b'='] | p => p,
    };

    const N: usize = 64;
    const N4: usize = 16;
    const N8: usize = 8;
    const { assert!(N8 * 2 == N4) };
    const { assert!(N4 * 4 == N) };

    for chunk in data.chunks(4) {
        let mut ascii = [b'A'; 4];
        ascii[..chunk.len()].copy_from_slice(chunk);

        let (bytes, ok) = decode_chunk_branched(ascii);
        if !ok {
            return Err(B64Error);
        }

        let len = decoded_len(chunk.len());
        out.extend_from_slice(&bytes[..len]);
    }

    Ok(())
}

/// Function yoinked from blogpost, adjusted to load 1 cache line at a time
pub fn unb64_nonsimd_cachelines(data: &[u8], out: &mut Vec<u8>) -> Result<(), B64Error> {
    let data = match data {
        [p @ .., b'=', b'='] | [p @ .., b'='] | p => p,
    };

    const N: usize = 64;
    const N4: usize = 16;
    const N8: usize = 8;
    const { assert!(N8 * 2 == N4) };
    const { assert!(N4 * 4 == N) };

    let (chunks, remainder) = data.as_chunks::<N>();

    let mut total_oks: u128 = 0x01010101_01010101_01010101_01010101;
    for chunk in chunks {
        let subchunks: [[u8; 4]; N4] = unsafe { transmute(*chunk) };

        let mut sextets = [const { MaybeUninit::<[u8; 3]>::uninit() }; N4];
        let mut oks = [const { MaybeUninit::<bool>::uninit() }; N4];
        for i in 0..N4 {
            let (bs, ok) = decode_chunk_branched(subchunks[i]);
            sextets[i] = MaybeUninit::new(bs);
            oks[i] = MaybeUninit::new(ok);
        }
        let outs = unsafe { sextets.transpose().assume_init() };

        let oks: u128 = unsafe { transmute(oks) };
        total_oks &= oks;
        out.extend_from_slice(outs.as_flattened());
    }

    for chunk in remainder.chunks(4) {
        let mut ascii = [b'A'; 4];
        ascii[..chunk.len()].copy_from_slice(chunk);

        let (bytes, ok) = decode_chunk_branched(ascii);
        if !ok {
            return Err(B64Error);
        }

        let len = decoded_len(chunk.len());
        out.extend_from_slice(&bytes[..len]);
    }

    if total_oks == 0x01010101_01010101_01010101_01010101 { Ok(()) } else { Err(B64Error) }
}

pub fn unb64_simd_elegant(data: &[u8], out: &mut Vec<u8>) -> Result<(), B64Error> {
    let data = match data {
        [p @ .., b'=', b'='] | [p @ .., b'='] | p => p,
    };

    const N: usize = 32;
    const N4: usize = 8;
    const { assert!(N4 * 4 == N) };

    let (chunks, remainder) = data.as_chunks::<N>();

    let mut total_ok = true;
    for chunk in chunks {
        let (sextets, ok) = decode_chunk_simd_elegant(Simd::from_array(*chunk));
        total_ok &= ok;
        let sextets: [[u8; 4]; N4] = unsafe { transmute(sextets) };
        // TODO: implement faster packing from blogpost
        let bytes = sextets.map(decode_pack);
        out.extend_from_slice(bytes.as_flattened());
    }

    for chunk in remainder.chunks(4) {
        let mut ascii = [b'A'; 4];
        ascii[..chunk.len()].copy_from_slice(chunk);

        let (bytes, ok) = decode_chunk_simd_elegant(u8x4::from_array(ascii));
        if !ok {
            return Err(B64Error);
        }
        let d = decode_pack(bytes.to_array());
        let len = decoded_len(chunk.len());
        out.extend_from_slice(&d[..len]);
    }

    if total_ok { Ok(()) } else { Err(B64Error) }
}

fn decode_chunk_branched(ascii: [u8; 4]) -> ([u8; 3], bool) {
    let mut bytes = 0u32;
    let mut ok = true;
    for byte in ascii {
        let sextet = match byte {
            b'A'..=b'Z' => byte - b'A',
            b'a'..=b'z' => byte - b'a' + 26,
            b'0'..=b'9' => byte - b'0' + 52,
            b'+' => 62,
            b'/' => 63,
            _ => 255,
        };

        bytes <<= 6;
        bytes |= sextet as u32;
        ok &= sextet != 255;
    }

    let [b1, b2, b3, _] = bytes.to_le_bytes();
    ([b3, b2, b1], ok)
}

fn in_range<const N: usize>(bytes: Simd<u8, N>, start: u8, end: u8) -> Mask<i8, N>
where
    LaneCount<N>: SupportedLaneCount,
{
    bytes.simd_ge(Simd::splat(start)) & bytes.simd_le(Simd::splat(end))
}

fn masked_splat<const N: usize>(mask: Mask<i8, N>, value: u8) -> Simd<u8, N>
where
    LaneCount<N>: SupportedLaneCount,
{
    mask.select(Simd::splat(value), Simd::splat(0))
}

/// Yoinked from blogpost
/// last byte of the vector is unspecified
fn decode_chunk_simd_elegant<const N: usize>(ascii: Simd<u8, N>) -> (Simd<u8, N>, bool)
where
    LaneCount<N>: SupportedLaneCount,
{
    // Create masks for each of the five ranges.
    // Note that these are disjoint: for any two masks, m1 & m2 == 0.
    let uppers = in_range(ascii, b'A', b'Z');
    let lowers = in_range(ascii, b'a', b'z');
    let digits = in_range(ascii, b'0', b'9');
    let pluses = ascii.simd_eq([b'+'; _].into());
    let solidi = ascii.simd_eq([b'/'; _].into());

    // If any byte was invalid, none of the masks will select for it,
    // so that lane will be 0 in the or of all the masks. This is our
    // validation check.
    let ok = (uppers | lowers | digits | pluses | solidi).all();

    // Given a mask, create a new vector by splatting `value`
    // over the set lanes.

    // Fill the the lanes of the offset vector by filling the
    // set lanes with the corresponding offset. This is like
    // a "vectorized" version of the `match`.
    let offsets = masked_splat(uppers, b'A')
        | masked_splat(lowers, b'a' - 26)
        | masked_splat(digits, b'0'.wrapping_sub(52))
        | masked_splat(pluses, b'+'.wrapping_sub(62))
        | masked_splat(solidi, b'/'.wrapping_sub(63));

    // Finally, Build the sextets vector.
    let sextets = ascii - offsets;

    (sextets, ok)
}

#[inline(always)]
const fn decode_pack(input: [u8; 4]) -> [u8; 3] {
    let mut output = 0u32;
    unroll!(4, |i| {
        output <<= 6;
        output |= input[i] as u32;
    });
    output <<= 8;
    let [b0, b1, b2, _] = output.to_be_bytes();
    [b0, b1, b2]
}

pub fn decoded_len(input: usize) -> usize {
    let mod4 = input % 4;
    input / 4 * 3 + (mod4 - mod4 / 2)
}

#[cfg(test)]
mod tests {
    use super::*;

    type Fun = unsafe fn(&[u8], &mut Vec<u8>) -> Result<(), B64Error>;
    const FUNS: &[(&str, Fun)] = &[
        ("unb64_nonsimd", unb64_nonsimd),
        ("unb64_nonsimd_cachelines", unb64_nonsimd_cachelines),
        ("unb64_simd_elegant", unb64_simd_elegant),
    ];

    #[test]
    fn test_unb64() {
        let source = include_bytes!("../test_data/b64_input.bin");
        let b64 = include_bytes!("../test_data/b64_expected_output.txt");
        let cap = decoded_len(b64.len());
        for (name, fun) in FUNS {
            let mut out = Vec::with_capacity(cap);
            let () = unsafe { fun(b64, &mut out).unwrap() };
            assert!(out.len() <= cap, "in fun {name}");
            assert_eq!(&out, source, "in fun {name}");
        }
    }

    #[test]
    fn test_unb64_wrong_input() {
        let b64 = [b'!'; 1000];
        let mut out = Vec::new();
        for (name, fun) in FUNS {
            assert!(unsafe { fun(&b64, &mut out).is_err() }, "in fun {name}");
        }
    }

    #[test]
    #[ignore = "slow"]
    fn test_chunks() {
        for n in 0..=u32::MAX {
            let ascii = n.to_le_bytes();
            let ([a0, a1, a2], ok1) = decode_chunk_branched(ascii);
            let (simd, ok2) = decode_chunk_simd_elegant(ascii.into());
            let [b0, b1, b2] = decode_pack(simd.to_array());
            if ok1 {
                assert!(ok2, "n={n}");
                assert_eq!([b0, b1, b2], [a0, a1, a2], "n={n}");
            } else {
                assert!(!ok2, "n={n}");
            }
        }
    }
}
