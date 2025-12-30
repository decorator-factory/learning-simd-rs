use core::simd::prelude::*;

pub fn count_spaces_std(src: &[u8]) -> u16 {
    let (line, _rest) = src.split_once(|c| *c == b'\n').unwrap();
    line.iter().filter(|c| **c == b' ').count() as u16
}

///  # Safety
/// - `src`` must be ASCII
/// - `src` must be at least 8 bytes large, must contain a '\n' and there must be at least 7 bytes after the first '\n'
/// - all characters before the first '\n' must be either ' ' or ASCII digits
/// - system must use little endian integers
pub unsafe fn count_spaces_simd_novec(src: &[u8]) -> u16 {
    let mut ptr = src.as_ptr();
    let mut total = 0u16;
    loop {
        let word: u64 = unsafe { core::ptr::read_unaligned(ptr.cast()) };
        let nl = word & 0x20202020_20202020;
        if nl != 0x20202020_20202020 {
            let tz = (nl ^ 0x20202020_20202020).trailing_zeros() - 5;
            let mask = 1u64.wrapping_shl(tz).wrapping_sub(1);
            let space_count = (tz / 8) - (word & mask & 0x10101010_10101010).count_ones();
            return total + space_count as u16;
        }
        total += 8 - ((word & 0x10101010_10101010).count_ones() as u16);
        ptr = unsafe { ptr.add(8) };
    }
}

///  # Safety
/// - `src`` must be ASCII
/// - `src` must be at least 64 bytes large, must contain a '\n' and there must be at least 63 bytes after the first '\n'
/// - all characters before the first '\n' must be either ' ' or ASCII digits
/// - system must use little endian integers
pub unsafe fn count_spaces_simd_novec_x8(src: &[u8]) -> u16 {
    let mut ptr = src.as_ptr();
    let mut total = 0u16;
    loop {
        // load an entire cache line at once
        let words: [u64; 8] = unsafe { core::ptr::read_unaligned(ptr.cast()) };
        let nls = words.map(|word| word & 0x20202020_20202020);
        let totals = words.map(|word| 8 - ((word & 0x10101010_10101010).count_ones() as u16));

        for (i, nl) in nls.into_iter().enumerate() {
            if nl != 0x20202020_20202020 {
                let tz = (nl ^ 0x20202020_20202020).trailing_zeros() - 5;
                let mask = 1u64.wrapping_shl(tz).wrapping_sub(1);
                let space_count =
                    (tz / 8) - (words[i] & mask & 0x10101010_10101010).count_ones();
                return total + space_count as u16;
            } else {
                total += totals[i];
            }
        }
        ptr = unsafe { ptr.add(64) };
    }
}

/// \[0, 1, ..., 31]
const INDICES32: u8x32 = {
    const fn usize_to_u8(i: usize) -> u8 {
        i as u8
    }
    Simd::from_array(core::array::from_fn(usize_to_u8))
};

///  # Safety
/// - `src` must be at least 64 bytes large, must contain a '\n' and there must be at least 63 bytes after the first '\n'
pub unsafe fn count_spaces_simd_portable_256(src: &[u8]) -> u16 {
    let mut ptr = src.as_ptr();
    let mut total = 0u16;

    const SPACES: u8x32 = Simd::splat(0x20);
    const NEWLINES: u8x32 = Simd::splat(0x0a);
    const ZEROS: u8x32 = Simd::splat(0x0);

    loop {
        // load an entire cache line at once
        let w1 = u8x32::from_array(unsafe { core::ptr::read_unaligned(ptr.cast()) });
        ptr = unsafe { ptr.add(32) };
        let w2 = u8x32::from_array(unsafe { core::ptr::read_unaligned(ptr.cast()) });
        ptr = unsafe { ptr.add(32) };

        let nl1 = w1.simd_eq(NEWLINES);
        let t1 = w1.simd_eq(SPACES).to_bitmask().count_ones() as u16;

        let nl2 = w2.simd_eq(NEWLINES);
        let t2 = w2.simd_eq(SPACES).to_bitmask().count_ones() as u16;

        if nl1.any() || nl2.any() {
            if let Some(idx) = nl1.first_set() {
                let before_newline = INDICES32.simd_lt(Simd::splat(idx as u8));
                total +=
                    before_newline.select(w1, ZEROS).simd_eq(SPACES).to_bitmask().count_ones()
                        as u16;
                return total;
            } else {
                let idx = nl2.first_set().unwrap();
                let before_newline = INDICES32.simd_lt(Simd::splat(idx as u8));
                total +=
                    before_newline.select(w2, ZEROS).simd_eq(SPACES).to_bitmask().count_ones()
                        as u16;
                return total + t1; // important: need to account for spaces from the first line
            }
        }

        total += t1 + t2;
    }
}

/// \[0, 1, ..., 63]
const INDICES64: u8x64 = {
    const fn usize_to_u8(i: usize) -> u8 {
        i as u8
    }
    Simd::from_array(core::array::from_fn(usize_to_u8))
};

///  # Safety
/// - `src` must be at least 64 bytes large, must contain a '\n' and there must be at least 63 bytes after the first '\n'
pub unsafe fn count_spaces_simd_portable_512(src: &[u8]) -> u16 {
    let mut ptr = src.as_ptr();
    let mut total = 0u16;

    const SPACES: u8x64 = Simd::splat(0x20);
    const NEWLINES: u8x64 = Simd::splat(0x0a);
    const ZEROS: u8x64 = Simd::splat(0x0);

    loop {
        let w = u8x64::from_array(unsafe { core::ptr::read_unaligned(ptr.cast()) });
        ptr = unsafe { ptr.add(64) };

        let nl = w.simd_eq(NEWLINES);

        if let Some(idx) = nl.first_set() {
            let before_newline = INDICES64.simd_lt(Simd::splat(idx as u8));
            total += before_newline.select(w, ZEROS).simd_eq(SPACES).to_bitmask().count_ones()
                as u16;
            return total;
        }
        let space_count = w.simd_eq(SPACES).to_bitmask().count_ones() as u16;
        total += space_count;
    }
}

// helper function
pub fn pad_bytes(src: &[u8]) -> &'static [u8] {
    assert!(src.contains(&b'\n'));
    let mut v = Vec::with_capacity(src.len() + 128);
    v.extend_from_slice(src);
    v.extend_from_slice(b"1 2 3 4 5 ");
    v.extend_from_slice(&[b'1'; 118]);
    Box::leak(v.into_boxed_slice())
}

#[cfg(test)]
mod test {
    use super::*;

    macro_rules! repr {
        ($e:expr) => {
            (stringify!($e), $e)
        };
    }

    #[test]
    fn all_of_them() {
        type Fn = unsafe fn(&[u8]) -> u16;

        let inputs: [&[u8]; _] = [
            b"\n",
            b"123\n",
            b"123456\n",
            b"1234567812345678123456781234567\n",
            b"123456781234567812345678123456789\n",
            b"1234567812345678123456781234567891\n",
            b"12345678123456781234567812345678912\n",
            b"1 2\n",
            b"1 2 3\n",
            b"1 2 3 4\n",
            b"1 2 3 4 5\n",
            b"1 2 3 4 5 6\n",
            b"1 2 3 4 5 6 7\n",
            include_bytes!("../test_data/count_spaces_long_input.txt"),
        ];
        let inputs = inputs.map(pad_bytes);

        let expected_outputs = inputs.map(count_spaces_std);

        let fns: [(&str, Fn); _] = [
            repr!(count_spaces_simd_novec),
            repr!(count_spaces_simd_novec_x8),
            repr!(count_spaces_simd_portable_256),
            repr!(count_spaces_simd_portable_512),
        ];

        for (repr, f) in fns {
            for (i, (&input, expected_output)) in
                inputs.iter().zip(expected_outputs).enumerate()
            {
                let output = unsafe { f(input) };
                assert_eq!(output, expected_output, "For function {repr} at input #{i:?}");
            }
        }
    }
}
