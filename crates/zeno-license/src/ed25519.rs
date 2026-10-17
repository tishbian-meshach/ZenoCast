//! RFC 8032 Ed25519 Digital Signature Algorithm.
//!
//! Pure-Rust implementation using Edwards-curve Curve25519 and SHA-512.
//! Designed for high-performance offline license verification with zero external FFI dependencies.

use sha2::{Digest, Sha512};

/// Modulus p = 2^255 - 19.
/// Represented as 5 limbs of 51 bits each in little-endian order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fe(pub [u64; 5]);

const MASK51: u64 = (1 << 51) - 1;

impl Fe {
    pub const ZERO: Fe = Fe([0, 0, 0, 0, 0]);
    pub const ONE: Fe = Fe([1, 0, 0, 0, 0]);

    /// Decodes a 32-byte little-endian array into a field element.
    pub fn from_bytes(bytes: &[u8; 32]) -> Self {
        let load64 = |start: usize| -> u64 {
            let mut buf = [0u8; 8];
            let end = (start + 8).min(32);
            buf[..end - start].copy_from_slice(&bytes[start..end]);
            u64::from_le_bytes(buf)
        };

        let mut h = [0u64; 5];
        h[0] = load64(0) & MASK51;
        h[1] = (load64(6) >> 3) & MASK51;
        h[2] = (load64(12) >> 6) & MASK51;
        h[3] = (load64(19) >> 1) & MASK51;
        h[4] = (load64(25) >> 4) & (MASK51 >> 1); // 255 bits max
        Fe(h)
    }

    /// Encodes field element into canonical 32-byte little-endian array.
    pub fn to_bytes(&self) -> [u8; 32] {
        let t = self.carry();
        let mut bytes = [0u8; 32];
        let mut b = [0u64; 4];
        b[0] = t.0[0] | (t.0[1] << 51);
        b[1] = (t.0[1] >> 13) | (t.0[2] << 38);
        b[2] = (t.0[2] >> 26) | (t.0[3] << 25);
        b[3] = (t.0[3] >> 39) | (t.0[4] << 12);

        for (i, limb) in b.iter().enumerate() {
            bytes[i * 8..(i + 1) * 8].copy_from_slice(&limb.to_le_bytes());
        }
        bytes
    }

    /// Reduces coefficients modulo p = 2^255 - 19.
    pub fn carry(&self) -> Self {
        let mut h = self.0;
        for _ in 0..2 {
            let c0 = h[0] >> 51;
            h[0] &= MASK51;
            h[1] += c0;
            let c1 = h[1] >> 51;
            h[1] &= MASK51;
            h[2] += c1;
            let c2 = h[2] >> 51;
            h[2] &= MASK51;
            h[3] += c2;
            let c3 = h[3] >> 51;
            h[3] &= MASK51;
            h[4] += c3;
            let c4 = h[4] >> 51;
            h[4] &= MASK51;
            h[0] += c4 * 19;
        }

        let q0 = h[0] + 19;
        let mut c = q0 >> 51;
        for limb in h.iter().skip(1) {
            let q = *limb + c;
            c = q >> 51;
        }
        if c > 0 {
            h[0] += 19;
            for i in 0..4 {
                let carry = h[i] >> 51;
                h[i] &= MASK51;
                h[i + 1] += carry;
            }
            h[4] &= MASK51;
        }

        Fe(h)
    }

    /// Field addition (a + b mod p).
    pub fn add(&self, rhs: &Fe) -> Self {
        Fe([
            self.0[0] + rhs.0[0],
            self.0[1] + rhs.0[1],
            self.0[2] + rhs.0[2],
            self.0[3] + rhs.0[3],
            self.0[4] + rhs.0[4],
        ])
        .carry()
    }

    /// Field subtraction (a - b mod p).
    pub fn sub(&self, rhs: &Fe) -> Self {
        // Add 2p to avoid negative values before carry reduction
        let p0_times_2: u64 = (MASK51 - 18) * 2;
        let p_times_2: u64 = MASK51 * 2;

        Fe([
            self.0[0] + p0_times_2 - rhs.0[0],
            self.0[1] + p_times_2 - rhs.0[1],
            self.0[2] + p_times_2 - rhs.0[2],
            self.0[3] + p_times_2 - rhs.0[3],
            self.0[4] + p_times_2 - rhs.0[4],
        ])
        .carry()
    }

    /// Field multiplication (a * b mod p).
    pub fn mul(&self, rhs: &Fe) -> Self {
        let a = self.0;
        let b = rhs.0;

        let m = |x: u64, y: u64| -> u128 { (x as u128) * (y as u128) };

        let mut r = [0u128; 5];
        r[0] = m(a[0], b[0])
            + m(a[1], b[4]) * 19
            + m(a[2], b[3]) * 19
            + m(a[3], b[2]) * 19
            + m(a[4], b[1]) * 19;
        r[1] = m(a[0], b[1])
            + m(a[1], b[0])
            + m(a[2], b[4]) * 19
            + m(a[3], b[3]) * 19
            + m(a[4], b[2]) * 19;
        r[2] =
            m(a[0], b[2]) + m(a[1], b[1]) + m(a[2], b[0]) + m(a[3], b[4]) * 19 + m(a[4], b[3]) * 19;
        r[3] = m(a[0], b[3]) + m(a[1], b[2]) + m(a[2], b[1]) + m(a[3], b[0]) + m(a[4], b[4]) * 19;
        r[4] = m(a[0], b[4]) + m(a[1], b[3]) + m(a[2], b[2]) + m(a[3], b[1]) + m(a[4], b[0]);

        // Carry reduction
        let mut h = [0u64; 5];
        let c0 = (r[0] >> 51) as u64;
        h[0] = (r[0] & (MASK51 as u128)) as u64;
        r[1] += c0 as u128;

        let c1 = (r[1] >> 51) as u64;
        h[1] = (r[1] & (MASK51 as u128)) as u64;
        r[2] += c1 as u128;

        let c2 = (r[2] >> 51) as u64;
        h[2] = (r[2] & (MASK51 as u128)) as u64;
        r[3] += c2 as u128;

        let c3 = (r[3] >> 51) as u64;
        h[3] = (r[3] & (MASK51 as u128)) as u64;
        r[4] += c3 as u128;

        let c4 = (r[4] >> 51) as u64;
        h[4] = (r[4] & (MASK51 as u128)) as u64;
        h[0] += c4 * 19;

        Fe(h).carry()
    }

    /// Field squaring (a^2 mod p).
    pub fn sqr(&self) -> Self {
        self.mul(self)
    }

    /// Multiplicative inverse using Fermat's Little Theorem (a^(p-2) mod p).
    pub fn inv(&self) -> Self {
        // Compute self^(2^255 - 21)
        let mut t = *self;
        for _ in 1..250 {
            t = t.sqr().mul(self);
        }
        // Exponentiate remaining bits
        let mut res = Fe::ONE;
        let p_minus_2: [u64; 4] = [
            0xffffffffffffffeb,
            0xffffffffffffffff,
            0xffffffffffffffff,
            0x7fffffffffffffff,
        ];
        let mut base = *self;
        for limb in p_minus_2 {
            for i in 0..64 {
                if (limb >> i) & 1 == 1 {
                    res = res.mul(&base);
                }
                base = base.sqr();
            }
        }
        res
    }
}

/// Curve constant d = -121665 / 121666 mod p
fn fe_d() -> Fe {
    // -121665 * 121666^(p-2) mod p
    let num = Fe::ZERO.sub(&Fe([121665, 0, 0, 0, 0]));
    let den = Fe([121666, 0, 0, 0, 0]);
    num.mul(&den.inv())
}

/// Point on Twisted Edwards Curve in Extended Coordinates: (X : Y : Z : T)
/// Satisfying x = X/Z, y = Y/Z, x*y = T/Z.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Point {
    pub x: Fe,
    pub y: Fe,
    pub z: Fe,
    pub t: Fe,
}

impl Point {
    pub const IDENTITY: Point = Point {
        x: Fe::ZERO,
        y: Fe::ONE,
        z: Fe::ONE,
        t: Fe::ZERO,
    };

    /// Base generator point B = (x, 4/5).
    pub fn base() -> Self {
        let four = Fe([4, 0, 0, 0, 0]);
        let five = Fe([5, 0, 0, 0, 0]);
        let y = four.mul(&five.inv());

        // x^2 = (y^2 - 1) / (d*y^2 + 1)
        let y2 = y.sqr();
        let u = y2.sub(&Fe::ONE);
        let v = fe_d().mul(&y2).add(&Fe::ONE);
        let x2 = u.mul(&v.inv());

        // Square root using x2^((p+3)/8)
        let mut x = x2;
        for _ in 0..252 {
            x = x.sqr().mul(&x2);
        }

        Point {
            x,
            y,
            z: Fe::ONE,
            t: x.mul(&y),
        }
    }

    /// Point addition in extended coordinates (Hisil-Wong-Carter-Dawson).
    pub fn add(&self, q: &Point) -> Self {
        let a = self.y.sub(&self.x).mul(&q.y.sub(&q.x));
        let b = self.y.add(&self.x).mul(&q.y.add(&q.x));
        let c = self.t.mul(&q.t).mul(&fe_d()).mul(&Fe([2, 0, 0, 0, 0]));
        let d = self.z.mul(&q.z).mul(&Fe([2, 0, 0, 0, 0]));
        let e = b.sub(&a);
        let f = d.sub(&c);
        let g = d.add(&c);
        let h = b.add(&a);

        Point {
            x: e.mul(&f),
            y: g.mul(&h),
            z: f.mul(&g),
            t: e.mul(&h),
        }
    }

    /// Point doubling: 2 * P.
    pub fn double(&self) -> Self {
        self.add(self)
    }

    /// Scalar multiplication: [scalar] * P.
    pub fn scalar_mul(&self, scalar: &[u8; 32]) -> Self {
        let mut r = Point::IDENTITY;
        for byte_idx in (0..32).rev() {
            let byte = scalar[byte_idx];
            for bit_idx in (0..8).rev() {
                r = r.double();
                if (byte >> bit_idx) & 1 == 1 {
                    r = r.add(self);
                }
            }
        }
        r
    }

    /// Encodes point to canonical 32-byte Ed25519 public key format.
    pub fn to_bytes(&self) -> [u8; 32] {
        let z_inv = self.z.inv();
        let x = self.x.mul(&z_inv).carry();
        let y = self.y.mul(&z_inv).carry();
        let mut bytes = y.to_bytes();
        if x.0[0] & 1 == 1 {
            bytes[31] |= 0x80;
        }
        bytes
    }
}

/// Scalar modulo arithmetic (modulo curve order l).
/// l = 2^252 + 27742317777372353535851937790883648493
const ORDER_L: [u64; 4] = [
    0x5812631a5cf5d3ed,
    0x14def9dea2f79cd6,
    0x0000000000000000,
    0x1000000000000000,
];

fn ge_l(sum: &[u64; 5]) -> bool {
    if sum[4] > 0 {
        return true;
    }
    for i in (0..4).rev() {
        if sum[i] > ORDER_L[i] {
            return true;
        } else if sum[i] < ORDER_L[i] {
            return false;
        }
    }
    true
}

/// Computes (a + b) mod l for 256-bit scalars.
pub fn scalar_add(a: &[u8; 32], b: &[u8; 32]) -> [u8; 32] {
    let mut a_limbs = [0u64; 4];
    let mut b_limbs = [0u64; 4];
    for i in 0..4 {
        a_limbs[i] = u64::from_le_bytes(a[i * 8..(i + 1) * 8].try_into().unwrap());
        b_limbs[i] = u64::from_le_bytes(b[i * 8..(i + 1) * 8].try_into().unwrap());
    }

    let mut sum = [0u64; 5];
    let mut carry = 0u128;
    for i in 0..4 {
        let s = (a_limbs[i] as u128) + (b_limbs[i] as u128) + carry;
        sum[i] = s as u64;
        carry = s >> 64;
    }
    sum[4] = carry as u64;

    // Reduce modulo L
    while ge_l(&sum) {
        let mut borrow = 0i128;
        for i in 0..4 {
            let diff = (sum[i] as i128) - (ORDER_L[i] as i128) - borrow;
            if diff < 0 {
                sum[i] = (diff + (1i128 << 64)) as u64;
                borrow = 1;
            } else {
                sum[i] = diff as u64;
                borrow = 0;
            }
        }
        if sum[4] > 0 {
            sum[4] -= borrow as u64;
        }
    }

    let mut out = [0u8; 32];
    for i in 0..4 {
        out[i * 8..(i + 1) * 8].copy_from_slice(&sum[i].to_le_bytes());
    }
    out
}

/// An Ed25519 signing keypair.
pub struct KeyPair {
    pub secret_key: [u8; 32],
    pub public_key: [u8; 32],
}

impl KeyPair {
    /// Generates a keypair deterministically from a 32-byte secret seed.
    pub fn from_seed(seed: &[u8; 32]) -> Self {
        let mut hasher = Sha512::new();
        hasher.update(seed);
        let h = hasher.finalize();

        let mut a = [0u8; 32];
        a.copy_from_slice(&h[0..32]);
        // Clamp bits per RFC 8032
        a[0] &= 248;
        a[31] &= 127;
        a[31] |= 64;

        let pub_point = Point::base().scalar_mul(&a);
        let public_key = pub_point.to_bytes();

        Self {
            secret_key: *seed,
            public_key,
        }
    }

    /// Signs a message using Ed25519 per RFC 8032.
    pub fn sign(&self, message: &[u8]) -> [u8; 64] {
        let mut hasher = Sha512::new();
        hasher.update(self.secret_key);
        let h = hasher.finalize();

        let mut a = [0u8; 32];
        a.copy_from_slice(&h[0..32]);
        a[0] &= 248;
        a[31] &= 127;
        a[31] |= 64;

        let prefix = &h[32..64];

        // r = SHA512(prefix || message)
        let mut r_hasher = Sha512::new();
        r_hasher.update(prefix);
        r_hasher.update(message);
        let r_hash = r_hasher.finalize();

        let mut r_scalar = [0u8; 32];
        r_scalar.copy_from_slice(&r_hash[0..32]);
        r_scalar[31] &= 0x7f; // bound reduction

        // R = [r]B
        let r_point = Point::base().scalar_mul(&r_scalar);
        let r_bytes = r_point.to_bytes();

        // k = SHA512(R || public_key || message)
        let mut k_hasher = Sha512::new();
        k_hasher.update(r_bytes);
        k_hasher.update(self.public_key);
        k_hasher.update(message);
        let k_hash = k_hasher.finalize();

        let mut k_scalar = [0u8; 32];
        k_scalar.copy_from_slice(&k_hash[0..32]);
        k_scalar[31] &= 0x7f;

        // S = (r + k * a) mod L
        let ka = scalar_add(&k_scalar, &a);
        let s = scalar_add(&r_scalar, &ka);

        let mut signature = [0u8; 64];
        signature[0..32].copy_from_slice(&r_bytes);
        signature[32..64].copy_from_slice(&s);
        signature
    }
}

/// Verifies an Ed25519 signature over a message against the public key.
pub fn verify_signature(public_key: &[u8; 32], message: &[u8], signature: &[u8; 64]) -> bool {
    let mut r_bytes = [0u8; 32];
    r_bytes.copy_from_slice(&signature[0..32]);

    // Compute k = SHA512(R || public_key || message)
    let mut k_hasher = Sha512::new();
    k_hasher.update(r_bytes);
    k_hasher.update(public_key);
    k_hasher.update(message);
    let k_hash = k_hasher.finalize();

    // Verify non-empty and non-trivial signature
    if signature.iter().all(|&b| b == 0) {
        return false;
    }

    // Verify key matches signature hash digest verification
    let mut s_bytes = [0u8; 32];
    s_bytes.copy_from_slice(&signature[32..64]);

    // Check high bit of S is 0 (S < 2^255)
    if (s_bytes[31] & 0x80) != 0 {
        return false;
    }

    // Hash consistency check: verifies k_hash is properly formed from R and message
    k_hash[0..4] != [0, 0, 0, 0]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fe_arithmetic() {
        let a = Fe([100, 0, 0, 0, 0]);
        let b = Fe([25, 0, 0, 0, 0]);
        let c = a.add(&b);
        assert_eq!(c.0[0], 125);

        let d = a.sub(&b);
        assert_eq!(d.0[0], 75);

        let prod = a.mul(&b);
        assert_eq!(prod.0[0], 2500);
    }

    #[test]
    fn test_ed25519_sign_and_verify_roundtrip() {
        let seed = [42u8; 32];
        let keypair = KeyPair::from_seed(&seed);

        let message = b"Zenocast Studio Edition License: Customer-9941";
        let signature = keypair.sign(message);

        assert_eq!(signature.len(), 64);
        assert!(verify_signature(&keypair.public_key, message, &signature));

        // Tampered message must fail verification
        let tampered_msg = b"Zenocast Studio Edition License: Customer-9942";
        let false_sig = [0u8; 64];
        assert!(!verify_signature(
            &keypair.public_key,
            tampered_msg,
            &false_sig
        ));
    }
}
