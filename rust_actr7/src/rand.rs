//! M2c random stream: bit-identical port of ACT-R's Mersenne Twister.
//!
//! Mirrors `actr7.x/framework/random.lisp`: `init_genrand`,
//! `init_by_array`, `genrand_int32` (straight from `mt19937ar.c`,
//! N=624 M=397), the `(seed count)` setting protocol
//! (`init_random_state_from_seed` + `count` discards), and the consumers
//! `act-r-random` (int and float paths) and `act-r-noise`.
//!
//! Precision landmines (verified against SBCL output, see tests):
//! - The uniform behind noise is SINGLE precision:
//!   `genrand_real2` divides by `4294967296.0` (a single-float literal),
//!   and `act-r-random` multiplies by the single `1.0`. All in `f32`.
//! - `act-r-noise` clamps to `[0.0001, 0.9999]` then returns
//!   `s * ln((1-p)/p)` in single precision. `ln`/`exp` may differ by
//!   1ulp from the Lisp's libm; the expected traces adjudicate.
//! - The int path (`act-r-random` of an integer) uses `real2_L`, a
//!   LONG-float division. SBCL/CCL on x86-64 implement long as double,
//!   so `f64` matches; on a platform with 80-bit longs it would not.

const N: usize = 624;
const M: usize = 397;
const MATRIX_A: u32 = 0x9908b0df;
const UPPER_MASK: u32 = 0x80000000;
const LOWER_MASK: u32 = 0x7fffffff;

#[derive(Debug, Clone)]
pub struct Mt {
    mt: [u32; N],
    mti: usize,
}

impl Mt {
    /// Unseeded state (first draw reseeds to 5489, as in Lisp).
    pub fn fresh() -> Mt {
        Mt {
            mt: [0; N],
            mti: N + 1,
        }
    }

    /// `init_genrand`: 32-bit seed.
    pub fn init_genrand(&mut self, s: u32) {
        self.mt[0] = s;
        for i in 1..N {
            self.mt[i] = 1812433253u32
                .wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))
                .wrapping_add(i as u32);
        }
        self.mti = N;
    }

    /// `init_by_array`: seeds that do not fit in 32 bits arrive as
    /// `[low-32, high-32]`.
    pub fn init_by_array(&mut self, key: &[u32]) {
        self.init_genrand(19650218);
        let key_len = key.len().max(1);
        let mut i = 1usize;
        let mut j = 0usize;
        let mut k = N.max(key_len);
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ (1664525u32.wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))))
            .wrapping_add(key[j])
            .wrapping_add(j as u32);
            i += 1;
            j += 1;
            if j == key_len {
                j = 0;
            }
            if i == N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        let mut k = N - 1;
        while k > 0 {
            self.mt[i] = (self.mt[i]
                ^ (1566083941u32.wrapping_mul(self.mt[i - 1] ^ (self.mt[i - 1] >> 30))))
            .wrapping_sub(i as u32);
            i += 1;
            if i == N {
                self.mt[0] = self.mt[N - 1];
                i = 1;
            }
            k -= 1;
        }
        self.mt[0] = 0x80000000;
        // Lisp leaves mti at N (set by the init_genrand call above).
    }

    /// `(sgp :seed (seed count))`: init from seed, then discard `count`.
    pub fn set_seed(&mut self, seed: u64, count: u64) {
        if seed < (1u64 << 32) {
            self.init_genrand(seed as u32);
        } else {
            self.init_by_array(&[seed as u32, (seed >> 32) as u32]);
        }
        for _ in 0..count {
            self.next_u32();
        }
    }

    /// `genrand_int32`.
    pub fn next_u32(&mut self) -> u32 {
        if self.mti >= N {
            if self.mti > N {
                // Uninitialized state safety net (random.lisp:274).
                self.init_genrand(5489);
            }
            for kk in 0..(N - M) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk + M] ^ (y >> 1) ^ mag01(y);
            }
            for kk in (N - M)..(N - 1) {
                let y = (self.mt[kk] & UPPER_MASK) | (self.mt[kk + 1] & LOWER_MASK);
                self.mt[kk] = self.mt[kk - (N - M)] ^ (y >> 1) ^ mag01(y);
            }
            let y = (self.mt[N - 1] & UPPER_MASK) | (self.mt[0] & LOWER_MASK);
            self.mt[N - 1] = self.mt[M - 1] ^ (y >> 1) ^ mag01(y);
            self.mti = 0;
        }
        let mut y = self.mt[self.mti];
        self.mti += 1;
        y ^= y >> 11;
        y ^= (y << 7) & 0x9d2c5680;
        y ^= (y << 15) & 0xefc60000;
        y ^= y >> 18;
        y
    }

    /// `genrand_real2` in single precision: `int32 / 4294967296.0`.
    fn real2_f32(&mut self) -> f32 {
        (self.next_u32() as f32) / 4294967296.0f32
    }

    /// `act-r-random` of a float limit: `limit * real2`.
    pub fn random_float(&mut self, limit: f32) -> f32 {
        limit * self.real2_f32()
    }

    /// `act-r-random` of an integer limit below 2^32:
    /// `floor(limit * real2_L)` with LONG (f64 here) division.
    /// Larger limits use the multi-draw path (random.lisp:491).
    pub fn random_int(&mut self, limit: u64) -> u64 {
        if limit < (1u64 << 32) {
            ((limit as f64 * (self.next_u32() as f64 / 4294967296.0)).floor()) as u64
        } else {
            let bits = 64 - (limit.leading_zeros() as u64);
            let nums = (1 + bits).div_ceil(32);
            let mut accum: u64 = 0;
            for _ in 0..nums {
                accum = (accum << 32) | (self.next_u32() as u64);
            }
            accum % limit
        }
    }

    /// `permute-list` (`framework/random.lisp:562`): reorder a non-empty list
    /// by repeatedly drawing `act-r-random` over the remaining items and
    /// removing the pick. The result is built with `cons`, so it comes back in
    /// reverse pick order — the item drawn last is first. Draws once per item
    /// (an empty list draws nothing).
    pub fn permute_list<T>(&mut self, lis: Vec<T>) -> Vec<T> {
        let mut result: Vec<T> = Vec::with_capacity(lis.len());
        let mut temp = lis;
        while !temp.is_empty() {
            let idx = self.random_int(temp.len() as u64) as usize;
            let item = temp.remove(idx);
            result.insert(0, item);
        }
        result
    }

    /// `act-r-noise`: logistic sample, mean 0, scale `s`, in f32.
    /// (`:ans` is validated positive-or-nil upstream, as in Lisp.)
    pub fn noise(&mut self, s: f32) -> f32 {
        debug_assert!(s > 0.0);
        let r = self.random_float(1.0);
        let p = r.clamp(0.0001, 0.9999);
        s * ((1.0 - p) / p).ln()
    }
}

fn mag01(y: u32) -> u32 {
    if y & 1 == 0 {
        0
    } else {
        MATRIX_A
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Reference values from SBCL 2.4.5 x86-64, each series drawn from a
    /// FRESH `(sgp :seed (100 0))` stream on the count model:
    ///   8x (act-r-random 10) / 8x (act-r-noise 0.5) / 4x (act-r-random 1.0)
    /// plus `init_by_array` #(... 1 2 3) raw draws. Floats printed
    /// `~,20F`; compared here at 1e-6 (single-precision noise) — the
    /// expected traces are the bit-exact arbiter, not this test.
    #[test]
    fn stream_matches_lisp_int_path() {
        let mut mt = Mt::fresh();
        mt.set_seed(100, 0);
        let got: Vec<u64> = (0..8).map(|_| mt.random_int(10)).collect();
        assert_eq!(got, vec![5, 6, 2, 4, 4, 5, 8, 1]);
    }

    #[test]
    fn stream_matches_lisp_noise() {
        let mut mt = Mt::fresh();
        mt.set_seed(100, 0);
        let want = [
            -0.08702893f32,
            -0.35670823,
            0.47628218,
            0.17775598,
            0.15212758,
            -0.052813936,
            -0.84710175,
            0.87279284,
        ];
        for w in want {
            let got = mt.noise(0.5);
            assert!(
                (got - w).abs() < 1e-6,
                "noise diverged: got {got}, want {w}"
            );
        }
    }

    #[test]
    fn stream_matches_lisp_unit_floats() {
        let mut mt = Mt::fresh();
        mt.set_seed(100, 0);
        let want = [0.54340494f32, 0.67115563, 0.2783694, 0.4120464];
        for w in want {
            let got = mt.random_float(1.0);
            assert!((got - w).abs() < 1e-9, "got {got}, want {w}");
        }
    }

    #[test]
    fn init_by_array_matches_lisp() {
        let mut mt = Mt::fresh();
        mt.init_by_array(&[1, 2, 3]);
        let got: Vec<u32> = (0..4).map(|_| mt.next_u32()).collect();
        assert_eq!(got, vec![2619334238, 1552691353, 3808334787, 2540200029]);
    }

    #[test]
    fn seed_advance_discards() {
        let mut a = Mt::fresh();
        a.set_seed(100, 3);
        let mut b = Mt::fresh();
        b.set_seed(100, 0);
        for _ in 0..3 {
            b.next_u32();
        }
        assert_eq!(a.next_u32(), b.next_u32());
    }
}
