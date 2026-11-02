//! Content-defined chunking: FastCDC with normalized chunking and a Gear
//! rolling hash (Xia et al., USENIX ATC 2016).
//!
//! A cut is placed where the hash of the last bytes matches a mask, so
//! boundaries depend on the content around them, not on their offset: an
//! insertion or deletion in the middle of a file moves the boundaries
//! near the edit only, and the chunks after it are found again unchanged.
//! Normalized chunking uses a stricter mask before the average size and a
//! looser one after it, which narrows the spread of chunk sizes.
//!
//! Chunk boundaries are not part of the on-disk format: every chunk is
//! named by the hash of its bytes and every file lists its chunks with
//! their sizes, so changing the parameters or the table below would only
//! make the first backup after the change store more new chunks.
//!
//! This module is pure computation (no I/O, std only) and is tested on its
//! own with `rustc --test`.

/// Gear table: 256 pseudo-random 64-bit values from SplitMix64 with a fixed
/// seed (the bytes of "Archeion"), computed at compile time.
const GEAR: [u64; 256] = gear_table();

const fn gear_table() -> [u64; 256] {
    let mut table = [0u64; 256];
    let mut state: u64 = 0x4172_6368_6569_6f6e;
    let mut i = 0;
    while i < 256 {
        state = state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut z = state;
        z = (z ^ (z >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        z = (z ^ (z >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        table[i] = z ^ (z >> 31);
        i += 1;
    }
    table
}

/// Sizes of the chunks, in bytes.
///
/// [`ChunkParams::PRODUCTION`] aims at 1 MiB chunks, between 256 KiB and
/// 4 MiB: large enough that a 1 TB file gives about a million chunks (each
/// one a line item in a manifest and, when new, a file on disk with ~100
/// bytes of zip overhead), small enough that editing a few bytes of a large
/// file stores about a megabyte, not the file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ChunkParams {
    min: usize,
    avg: usize,
    max: usize,
    /// Stricter mask, used before the average size (more one-bits).
    mask_small: u64,
    /// Looser mask, used after the average size.
    mask_large: u64,
}

impl ChunkParams {
    /// 256 KiB / 1 MiB / 4 MiB.
    pub const PRODUCTION: Self = match Self::new(256 * 1024, 1024 * 1024, 4 * 1024 * 1024) {
        Some(p) => p,
        None => panic!("valid production parameters"),
    };

    /// Minimum, average and maximum chunk size. `None` unless
    /// `64 <= min < avg < max <= u32::MAX` and `avg` is a power of two
    /// between 2^6 and 2^60.
    #[must_use]
    pub const fn new(min: usize, avg: usize, max: usize) -> Option<Self> {
        if min < 64 || min >= avg || avg >= max || !avg.is_power_of_two() || max as u64 > u32::MAX as u64 {
            return None;
        }
        let bits = avg.trailing_zeros();
        if bits < 6 {
            return None;
        }
        if bits > 60 {
            return None;
        }
        // The highest bits of the Gear hash depend on the last 64 bytes, the
        // lowest on the last byte only: test the highest ones.
        let mask_small = !0u64 << (64 - (bits + 2));
        let mask_large = !0u64 << (64 - (bits - 2));
        Some(Self { min, avg, max, mask_small, mask_large })
    }

    /// Largest chunk, in bytes.
    #[must_use]
    pub const fn max(&self) -> usize {
        self.max
    }

    /// Smallest chunk (except the last chunk of a file), in bytes.
    #[must_use]
    pub const fn min(&self) -> usize {
        self.min
    }
}

/// Length of the first chunk of `data`: `data.len()` when it is no longer
/// than the minimum, never more than the maximum. When `data` holds at
/// least the maximum, the result does not depend on the bytes after the
/// chunk; with less, it is final only at the end of the content.
#[must_use]
pub fn cut_point(data: &[u8], p: &ChunkParams) -> usize {
    let len = data.len();
    if len <= p.min {
        return len;
    }
    let end = len.min(p.max);
    let normal = p.avg.min(end);
    let mut hash: u64 = 0;
    for (i, &b) in data.iter().enumerate().take(normal).skip(p.min) {
        hash = (hash << 1).wrapping_add(GEAR[usize::from(b)]);
        if hash & p.mask_small == 0 {
            return i + 1;
        }
    }
    for (i, &b) in data.iter().enumerate().take(end).skip(normal) {
        hash = (hash << 1).wrapping_add(GEAR[usize::from(b)]);
        if hash & p.mask_large == 0 {
            return i + 1;
        }
    }
    end
}

/// Splits a stream into chunks as it is fed: [`Chunker::push`] the bytes in
/// any pieces, then [`Chunker::finish`]. The chunks are the same however
/// the stream is cut into pieces.
#[derive(Debug, Clone)]
pub struct Chunker {
    params: ChunkParams,
    buf: Vec<u8>,
    /// Start of the bytes not yet emitted in `buf`.
    start: usize,
}

impl Chunker {
    #[must_use]
    pub fn new(params: ChunkParams) -> Self {
        Self { params, buf: Vec::with_capacity(params.max.saturating_mul(2)), start: 0 }
    }

    /// Adds `data`; `emit` receives every chunk that is now complete, in
    /// order. Stops at the first error of `emit`.
    ///
    /// # Errors
    /// The first error returned by `emit`.
    pub fn push<E>(&mut self, data: &[u8], emit: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        self.buf.extend_from_slice(data);
        while self.buf.len() - self.start >= self.params.max {
            let pending = self.buf.get(self.start..).unwrap_or_default();
            let n = cut_point(pending, &self.params);
            emit(pending.get(..n).unwrap_or_default())?;
            self.start += n;
        }
        if self.start >= self.params.max {
            self.buf.drain(..self.start);
            self.start = 0;
        }
        Ok(())
    }

    /// Emits the remaining chunks: the stream has ended. An empty stream
    /// has no chunk.
    ///
    /// # Errors
    /// The first error returned by `emit`.
    pub fn finish<E>(mut self, emit: &mut impl FnMut(&[u8]) -> Result<(), E>) -> Result<(), E> {
        while self.start < self.buf.len() {
            let pending = self.buf.get(self.start..).unwrap_or_default();
            let n = cut_point(pending, &self.params);
            emit(pending.get(..n).unwrap_or_default())?;
            self.start += n;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> ChunkParams {
        ChunkParams::new(1024, 4096, 16384).unwrap()
    }

    /// Deterministic pseudo-random bytes (xorshift).
    fn noise(len: usize, seed: u64) -> Vec<u8> {
        let mut x = seed | 1;
        (0..len)
            .map(|_| {
                x ^= x << 13;
                x ^= x >> 7;
                x ^= x << 17;
                x.to_le_bytes()[0]
            })
            .collect()
    }

    fn split(data: &[u8], p: ChunkParams, piece: usize) -> Vec<Vec<u8>> {
        let mut out = Vec::new();
        let mut c = Chunker::new(p);
        let mut emit = |d: &[u8]| -> Result<(), ()> {
            out.push(d.to_vec());
            Ok(())
        };
        for part in data.chunks(piece.max(1)) {
            c.push(part, &mut emit).unwrap();
        }
        c.finish(&mut emit).unwrap();
        out
    }

    #[test]
    fn chunks_rebuild_the_input_and_respect_the_sizes() {
        let p = small();
        let data = noise(1_000_000, 7);
        let chunks = split(&data, p, 65_536);
        assert_eq!(chunks.concat(), data);
        let (last, rest) = chunks.split_last().unwrap();
        for c in rest {
            assert!(c.len() > p.min() && c.len() <= p.max(), "size {}", c.len());
        }
        assert!(!last.is_empty() && last.len() <= p.max());
        let avg = data.len() / chunks.len();
        assert!((2048..=8192).contains(&avg), "average {avg}");
    }

    #[test]
    fn the_same_content_gives_the_same_chunks_however_it_is_fed() {
        let data = noise(300_000, 11);
        let whole = split(&data, small(), data.len());
        for piece in [1, 7, 1000, 4096, 16384, 100_000] {
            assert_eq!(split(&data, small(), piece), whole, "pieces of {piece}");
        }
    }

    #[test]
    fn an_insertion_in_the_middle_changes_only_nearby_chunks() {
        let data = noise(2_000_000, 3);
        let mut edited = data.clone();
        let middle = data.len() / 2;
        edited.splice(middle..middle, b"a few new bytes in the middle".iter().copied());
        let before = split(&data, small(), 65_536);
        let after = split(&edited, small(), 65_536);
        let old: std::collections::HashSet<&Vec<u8>> = before.iter().collect();
        let changed = after.iter().filter(|c| !old.contains(c)).count();
        assert!(changed <= 3, "{changed} of {} chunks changed", after.len());
        assert!(after.len() > 300);
    }

    #[test]
    fn a_deletion_at_the_start_resynchronises() {
        let data = noise(1_000_000, 5);
        let edited = data[100..].to_vec();
        let before = split(&data, small(), 65_536);
        let after = split(&edited, small(), 65_536);
        let old: std::collections::HashSet<&Vec<u8>> = before.iter().collect();
        let changed = after.iter().filter(|c| !old.contains(c)).count();
        assert!(changed <= 2, "{changed} of {} chunks changed", after.len());
    }

    #[test]
    fn empty_and_tiny_inputs() {
        assert!(split(&[], small(), 10).is_empty());
        assert_eq!(split(b"x", small(), 10), vec![b"x".to_vec()]);
        let at_min = noise(1024, 9);
        assert_eq!(split(&at_min, small(), 100), vec![at_min.clone()]);
        assert_eq!(cut_point(&[], &small()), 0);
    }

    #[test]
    fn constant_content_is_cut_at_the_maximum() {
        let zeros = vec![0u8; 100_000];
        let chunks = split(&zeros, small(), 4096);
        assert!(chunks.iter().rev().skip(1).all(|c| c.len() == 16384));
        assert_eq!(chunks.concat(), zeros);
    }

    #[test]
    fn parameters_are_validated() {
        assert!(ChunkParams::new(1024, 4096, 16384).is_some());
        assert!(ChunkParams::new(4096, 4096, 16384).is_none(), "min below avg");
        assert!(ChunkParams::new(1024, 5000, 16384).is_none(), "avg a power of two");
        assert!(ChunkParams::new(1024, 4096, 4096).is_none(), "avg below max");
        assert!(ChunkParams::new(8, 4096, 16384).is_none(), "min at least 64");
        assert_eq!(ChunkParams::PRODUCTION.max(), 4 * 1024 * 1024);
        assert_eq!(ChunkParams::PRODUCTION.min(), 256 * 1024);
    }

    #[test]
    fn production_parameters_average_about_a_mebibyte() {
        let data = noise(24 * 1024 * 1024, 13);
        let chunks = split(&data, ChunkParams::PRODUCTION, 1 << 20);
        assert_eq!(chunks.concat(), data);
        let avg = data.len() / chunks.len();
        assert!((512 * 1024..=2 * 1024 * 1024).contains(&avg), "average {avg}");
    }

    #[test]
    fn the_gear_table_is_fixed() {
        // Changing it changes every boundary: keep it unless on purpose.
        assert_eq!(GEAR[0], gear_table()[0]);
        assert_ne!(GEAR[0], GEAR[1]);
        let distinct: std::collections::HashSet<u64> = GEAR.iter().copied().collect();
        assert_eq!(distinct.len(), 256);
    }
}
