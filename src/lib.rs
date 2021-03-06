
#[cfg(test)] #[macro_use] extern crate quickcheck;

#[cfg(test)] use quickcheck::Arbitrary;
#[cfg(test)] use quickcheck::Gen;

use std::cmp::max;
use std::cmp::min;
use std::fmt;
use std::ops::Div;
use std::ops::Add;
use std::ops::Rem;
use std::ops::Mul;
use std::ops::Sub;
use std::cmp::Ordering;
use std::cmp::Ord;
use std::iter::Sum;

/// Little-endian arbitrarily-sized unsigned integer
#[derive(Eq, Debug, Clone)]
pub struct NonSmallInt { digits: Vec<u8> }

const RADIX: u64 = 10;

impl NonSmallInt {

    /// Constructs from a u64
    pub fn of(n: u64) -> NonSmallInt {
        let str_digits = format!("{}", n);
        NonSmallInt::parse(&str_digits).unwrap()
    }

    /// Parses from a radix 10 number
    pub fn parse(n: &str) -> Option<NonSmallInt> {
        let mut digits = Vec::new();
        let mut is_number = true;
        for c in n.trim().chars() {
            if c.is_digit(RADIX as u32) {
                digits.push(c.to_digit(RADIX as u32).expect("Failed to parse digit") as u8)
            } else {
                is_number = false;
            }
        };
        digits.reverse();

        if is_number {
            Some(NonSmallInt { digits: digits})
        } else {
            None
        }
    }

    /// Number of significant digits
    pub fn length(&self, radix: u64) -> usize {
        if radix == RADIX {
            self.digits.iter().rev().skip_while(|&n| *n == 0).count()
        } else {
            panic!("Unsupported feature: computing length of different radix")
        }
    }

    /// Multiplies by RADIX^n
    pub fn times_radix(&self, n: usize) -> NonSmallInt {
        let mut out = self.digits.clone();
        for _ in 0..n {
            out.insert(0, 0);
        }
        NonSmallInt { digits: out }
    }

    pub fn pow(&self, n: u32) -> NonSmallInt {
        if n == 0 {
            NonSmallInt::of(1)
        } else {
            self * self.pow(n-1)
        }
    }

    pub fn is_zero(&self) -> bool {
        self.digits.len() == 0 || self.digits.iter().all(|&n| n == 0)
    }

    /// Returns (quotient, remainder)
    fn div_u32(&self, rhs: u32) -> Option<(NonSmallInt, NonSmallInt)> {
        if rhs == 0 {
            None
        } else {
            let mut quotient = Vec::new();
            let mut carry = 0u64;
            for digit in self.digits.iter().rev() {
                let temp: u64 = carry * RADIX + (*digit as u64);
                let out: u8 = (temp / rhs as u64) as u8;
                carry = temp % (rhs as u64);
                quotient.insert(0, out);
            }
            let mut remainder = Vec::new();
            while carry > 0 {
                let out = carry % RADIX;
                carry = carry / RADIX;
                remainder.push(out as u8);
            }
            Some((NonSmallInt { digits: quotient }, NonSmallInt { digits: remainder }))
        }
    }

    fn div_nsi(&self, rhs: &NonSmallInt) -> Option<(NonSmallInt, NonSmallInt)> {
        if rhs.is_zero() {
            None
        } else if rhs.length(RADIX) == 1 {
            self.div_u32(rhs.digits[0] as u32)
        } else if self.length(RADIX) < rhs.length(RADIX) {
            Some((NonSmallInt { digits: vec![] }, self.clone()))
        } else {
            long_division(self, rhs)
        }
    }

    fn lt(&self, rhs: &NonSmallInt) -> bool {
        if self.length(RADIX) < rhs.length(RADIX) {
            true
        } else {
            let max_length = max(self.digits.len(), rhs.digits.len());
            let lhs_digits = self.iter_digits(max_length).rev();
            let rhs_digits = rhs.iter_digits(max_length).rev();
            match lhs_digits.zip(rhs_digits).skip_while(|&(lhs_d, rhs_d)| lhs_d == rhs_d).next() {
                None => false,
                Some((lhs_d, rhs_d)) => lhs_d < rhs_d
            }
        }
    }

    /// Result or None for underflow
    fn safe_sub(&self, rhs: &NonSmallInt) -> Option<NonSmallInt> {
        let mut out = Vec::new();
        let mut borrow = 0u32;
        let max_length = max(self.digits.len(), rhs.digits.len());
        for (l, r) in self.iter_digits(max_length).zip(rhs.iter_digits(max_length)) {
            let diff: u32 = (RADIX as u32 + l as u32).wrapping_sub(r as u32 + borrow);
            out.push((diff % RADIX as u32) as u8);
            borrow = 1 - diff / RADIX as u32;
        }
        if borrow == 0 {
            Some(NonSmallInt { digits: out })
        } else {
            None
        }
    }

    fn iter_digits(&self, length: usize) -> Digits {
        Digits { nsi: self, next_ix: 0, next_back_ix: length as isize - 1, empty: length == 0 }
    }
}

struct Digits<'a> { nsi: &'a NonSmallInt, next_ix: usize, next_back_ix: isize, empty: bool }

impl <'a> Iterator for Digits<'a> {
    type Item = u8;
    fn next(&mut self) -> Option<u8> {
        let next_value = |d: &mut Digits| {
            let out = if d.next_ix < d.nsi.digits.len() { d.nsi.digits[d.next_ix] } else { 0 };
            d.next_ix += 1;
            out
        };
        if !self.empty && self.next_ix <= self.next_back_ix as usize {
            Some(next_value(self))
        } else {
            None
        }
    }
}

impl <'a> DoubleEndedIterator for Digits<'a> {
    fn next_back(&mut self) -> Option<u8> {
        let next_value = |d: &mut Digits| {
            let out = if (d.next_back_ix as usize) < d.nsi.digits.len() { d.nsi.digits[d.next_back_ix as usize] } else { 0 };
            d.next_back_ix -= 1;
            out
        };
        if !self.empty && self.next_back_ix >= (self.next_ix as isize) {
            Some(next_value(self))
        } else {
            None
        }
    }
}

/// Implementation from http://surface.syr.edu/cgi/viewcontent.cgi?article=1162&context=eecs_techreports
/// Requires 2 <= rhs.length() <= lhs.length()
fn long_division(lhs: &NonSmallInt, rhs: &NonSmallInt) -> Option<(NonSmallInt, NonSmallInt)> {

    if rhs.is_zero() {
        None
    } else {
        trait IndexingIsHard<A> {
            /// Returns default value for A if doesn't exist
            fn lookup(&self, ix: usize) -> A;

            /// Resizes self if doesn't fit new value
            fn put(&mut self, ix: usize, value: A);
        }

        static ZERO: u8 = 0;

        impl IndexingIsHard<u8> for Vec<u8> {
            fn lookup(&self, ix: usize) -> u8 {
                *self.get(ix).unwrap_or(&ZERO)
            }
            fn put(&mut self, ix: usize, value: u8) {
                if ix < self.len() {
                    self[ix] = value;
                } else {
                    self.insert(ix, value);
                }
            }
        }

        let trial = |r: &Vec<u8>, d: &Vec<u8>, k: usize, m: usize| -> u8 {
            let km = k + m;
            let r3: u64 = ((r.lookup(km) as u64 * RADIX) + r.lookup(km-1) as u64) * RADIX + r.lookup(km-2) as u64;
            let d2: u64 = d.lookup(m-1) as u64 * RADIX + d.lookup(m-2) as u64;
            min(r3 / d2, RADIX - 1) as u8
        };

        let smaller = |r: &Vec<u8>, dq: &Vec<u8>, k: usize, m: usize| -> bool {
            let mut i = m;
            let mut j = 0;
            while i != j {
                if r.lookup(i+k) != dq.lookup(i) {
                    j = i;
                } else {
                    i = i - 1;
                }
            }
            r.lookup(i+k) < dq.lookup(i)
        };

        let difference = |r: &mut Vec<u8>, dq: &Vec<u8>, k: usize, m: usize| {
            let mut borrow: u64 = 0;
            for i in 0..m+1 {
                let diff: u64 = (RADIX + *r.get(i+k).unwrap_or(&ZERO) as u64).wrapping_sub(*dq.get(i).unwrap_or(&ZERO) as u64 + borrow);
                if (i+k) < r.len() {
                    r[i+k] = (diff % RADIX) as u8;
                } else {
                    r.insert(i+k, (diff % RADIX) as u8);
                }
                borrow = 1 - diff / RADIX;
            }
        };

        let longdivide = |x: &NonSmallInt, y: &NonSmallInt| -> (NonSmallInt, NonSmallInt) {
            let n = x.length(RADIX);
            let m = y.length(RADIX);

            let f: u8 = RADIX as u8 / (y.digits[m-1] + 1);

            let mut r = x * f as u32;
            let d = y * f as u32;
            let mut q = Vec::new();

            for k in (0..(n-m+1)).rev() {
                let mut qt = trial(&r.digits, &d.digits, k, m);
                let mut dq = &d * qt as u32;
                if smaller(&r.digits, &dq.digits, k, m) {
                    qt = qt - 1;
                    dq = &d * qt as u32;
                }
                q.insert(0, qt as u8);
                difference(&mut r.digits, &dq.digits, k, m)
            }

            r = r.div_u32(f as u32).expect("Division by Zero is not permitted").0;

            (NonSmallInt { digits: q }, r)
        };

        Some(longdivide(lhs, rhs))
    }
}

impl PartialEq for NonSmallInt {
    fn eq(&self, other: &NonSmallInt) -> bool {
        self.digits.iter().rev().skip_while(|&n| *n == 0).eq(other.digits.iter().rev().skip_while(|&n| *n == 0))
    }
}

impl <'a> Div for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn div(self, rhs: &NonSmallInt) -> NonSmallInt {
        match self.div_nsi(&rhs) {
            None => panic!("Division by zero is not allowed"),
            Some((q, _)) => q
        }
    }
}

impl <'a> Div<u32> for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn div(self, rhs: u32) -> NonSmallInt {
        match self.div_u32(rhs) {
            None => panic!("Division by zero is not allowed"),
            Some((q, _)) => q
        }
    }
}

impl <'a> Rem for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn rem(self, rhs: &NonSmallInt) -> NonSmallInt {
        match self.div_nsi(rhs) {
            None => panic!("Division by zero is not supported"),
            Some((_, r)) => r
        }
    }
}

impl <'a> Rem<u32> for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn rem(self, rhs: u32) -> NonSmallInt {
        match self.div_u32(rhs) {
            None => panic!("Division by zero is not supported"),
            Some((_, r)) => r
        }
    }
}

impl <'a> Mul<u32> for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn mul(self, rhs: u32) -> NonSmallInt {
        let mut out_digits = Vec::new();
        let mut carry = 0u64;
        for digit in self.digits.iter() {
            let temp: u64 = (rhs as u64) * (*digit as u64) + carry;
            let out: u8 = (temp % RADIX) as u8;
            carry = temp / RADIX;
            out_digits.push(out);
        }
        while carry != 0 {
            let out: u8 = (carry % RADIX) as u8;
            carry = carry / RADIX;
            out_digits.push(out);
        }
        NonSmallInt { digits: out_digits }
    }
}

impl Mul<u32> for NonSmallInt {
    type Output = NonSmallInt;
    fn mul(self, rhs: u32) -> NonSmallInt {
        (&self).mul(rhs)
    }
}

impl <'a> Mul for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn mul(self, rhs: &NonSmallInt) -> NonSmallInt {
        let mut out = NonSmallInt::of(0);
        for (&rhs_d, ix) in rhs.digits.iter().zip(0..) {
            let to_be_added = (self * (rhs_d as u32)).times_radix(ix);
            out = out + to_be_added;
        }
        out
    }
}

impl <'a> Mul<NonSmallInt> for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn mul(self, rhs: NonSmallInt) -> NonSmallInt {
        self.mul(&rhs)
    }
}

impl Mul for NonSmallInt {
    type Output = NonSmallInt;
    fn mul(self, rhs: NonSmallInt) -> NonSmallInt {
        (&self).mul(&rhs)
    }
}

impl <'a> Sub for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn sub(self, rhs: &NonSmallInt) -> NonSmallInt {
        match self.safe_sub(rhs) {
            Some(r) => r,
            None => panic!("NonSmallInt underflow")
        }
    }
}

impl <'a> Add for &'a NonSmallInt {
    type Output = NonSmallInt;
    fn add(self, rhs: &NonSmallInt) -> NonSmallInt {
        let mut out = Vec::new();
        let mut carry = 0u32;
        let max_length = max(self.length(RADIX), rhs.length(RADIX));
        for (ld, rd) in self.iter_digits(max_length).zip(rhs.iter_digits(max_length)) {
            let temp: u32 = ld as u32 + rd as u32 + carry;
            out.push((temp % RADIX as u32) as u8);
            carry = temp / RADIX as u32;
        }
        if carry != 0 {
            out.push((carry % RADIX as u32) as u8);
        }
        NonSmallInt { digits: out }
    }
}

impl Add for NonSmallInt {
    type Output = NonSmallInt;
    fn add(self, rhs: NonSmallInt) -> NonSmallInt {
        (&self).add(&rhs)
    }
}

impl PartialOrd for NonSmallInt {
    fn partial_cmp(&self, other: &NonSmallInt) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for NonSmallInt {
    fn cmp(&self, other: &NonSmallInt) -> Ordering {
        if self.lt(other) {
            Ordering::Less
        } else if self == other {
            Ordering::Equal
        } else {
            Ordering::Greater
        }
    }
}

impl fmt::Display for NonSmallInt {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.is_zero() {
            write!(f, "0")
        } else {
            let mut result = write!(f, "");
            for d in self.digits.iter().rev() {
                result = write!(f, "{}", d);
            }
            result
        }
    }
}

impl Sum for NonSmallInt {
    fn sum<I>(iter: I) -> NonSmallInt where I: Iterator<Item = NonSmallInt> {
        let mut acc = NonSmallInt::of(0);
        for x in iter {
            acc = acc + x;
        }
        acc
    }
}

#[cfg(test)]
mod tests {

    use super::*;

    impl Arbitrary for NonSmallInt {
        fn arbitrary<G: Gen>(g: &mut G) -> NonSmallInt {
            NonSmallInt::of(u64::arbitrary(g))
        }
    }

    #[derive(Clone, Debug)]
    /// A NonSmallInt along with the same value as u64
    struct MinimalNonSmallInt { nsi: NonSmallInt, n: u64 }

    impl MinimalNonSmallInt {
        fn of(n: u64) -> MinimalNonSmallInt {
            MinimalNonSmallInt { nsi: NonSmallInt::of(n), n: n }
        }
    }

    impl Arbitrary for MinimalNonSmallInt {
        fn arbitrary<G: Gen>(g: &mut G) -> MinimalNonSmallInt {
            let n = u64::arbitrary(g);
            MinimalNonSmallInt::of(n)
        }
    }

    #[derive(Clone, Debug)]
    struct SmallInt { n: u8 }

    impl Arbitrary for SmallInt {
        fn arbitrary<G: Gen>(g: &mut G) -> SmallInt {
            let n = u8::arbitrary(g) % 10;
            SmallInt { n: n }
        }
    }

    quickcheck! {

        fn counts_length_correctly(x: MinimalNonSmallInt) -> bool {
            if x.n > 0 {
                x.nsi.length(RADIX) == format!("{}", x.n).len()
            } else {
                x.nsi.length(RADIX) == 0
            }
        }

        fn comparison(x: MinimalNonSmallInt, y: MinimalNonSmallInt) -> bool {
            x.n.cmp(&y.n) == x.nsi.cmp(&y.nsi)
        }

        fn multiplies_by_u32(x: u32, y: u32) -> bool {
            let xnsi = NonSmallInt::of(x as u64);
            let expected = NonSmallInt::of((x * y) as u64);

            &xnsi * y == expected
        }

        fn div_by_u32(x: MinimalNonSmallInt, y: u32) -> bool {
            if y != 0 {
                x.nsi.div_u32(y) == Some((NonSmallInt::of(x.n / y as u64), NonSmallInt::of(x.n % y as u64)))
            } else {
                x.nsi.div_u32(y) == None
            }
        }

        fn subtracts(x: MinimalNonSmallInt, y: MinimalNonSmallInt) -> bool {
            if x.n >= y.n {
                x.nsi.safe_sub(&y.nsi).unwrap() == NonSmallInt::of(x.n - y.n)
            } else {
                x.nsi.safe_sub(&y.nsi).is_none()
            }
        }

        fn full_division(x: MinimalNonSmallInt, y: MinimalNonSmallInt) -> bool {
            let result = x.nsi.div_nsi(&y.nsi);
            if y.n != 0 {
                result == Some((NonSmallInt::of(x.n / y.n), NonSmallInt::of(x.n % y.n)))
            } else {
                result == None
            }
        }

        fn displays(x: MinimalNonSmallInt) -> bool {
            format!("{}", x.nsi) == format!("{}", x.n)
        }

        fn div_operator(x: MinimalNonSmallInt, y: MinimalNonSmallInt) -> bool {
            if y.n != 0 {
                NonSmallInt::of(x.n / y.n) == (&x.nsi / &y.nsi)
            } else {
                true
            }
        }

        fn rem_operator(x: MinimalNonSmallInt, y: MinimalNonSmallInt) -> bool {
            if y.n != 0 {
                NonSmallInt::of(x.n % y.n) == (&x.nsi % &y.nsi)
            } else {
                true
            }
        }

        fn add_operator(x: MinimalNonSmallInt, y: MinimalNonSmallInt) -> bool {
            let lhs = NonSmallInt::of(x.n + y.n);
            let rhs = &x.nsi + &y.nsi;
            lhs == rhs
        }

        fn mul_operator(x: MinimalNonSmallInt, y: MinimalNonSmallInt) -> bool {
            let lhs = NonSmallInt::of(x.n * y.n);
            let rhs = x.nsi * y.nsi;
            lhs == rhs
        }

        fn power(x: MinimalNonSmallInt, y: SmallInt) -> bool {
            x.nsi.pow(y.n as u32) == NonSmallInt::of(x.n.pow(y.n as u32))
        }

        fn sum(xs: Vec<MinimalNonSmallInt>) -> bool {
            let smallsies: Vec<u64> = xs.iter().map(|n| n.n).collect();
            let bigsies: Vec<NonSmallInt> = xs.into_iter().map(|n| n.nsi).collect();

            NonSmallInt::of(smallsies.iter().sum()) == bigsies.into_iter().sum()
        }
    }

    #[test]
    fn double_sided_iter_digits() {
        let nsi = NonSmallInt::of(654321);
        let mut iter = nsi.iter_digits(10);

        assert_eq!(Some(0), iter.next_back());
        assert_eq!(Some(0), iter.next_back());
        assert_eq!(Some(0), iter.next_back());
        assert_eq!(Some(0), iter.next_back());
        assert_eq!(Some(1), iter.next());
        assert_eq!(Some(6), iter.next_back());
        assert_eq!(Some(5), iter.next_back());
        assert_eq!(Some(2), iter.next());
        assert_eq!(Some(3), iter.next());
        assert_eq!(Some(4), iter.next());
        assert_eq!(None, iter.next());
        assert_eq!(None, iter.next_back());

        let reversed: Vec<u8> = nsi.iter_digits(6).rev().collect();
        let reversed_expected: Vec<u8> = (1..7).rev().collect();
        assert_eq!(reversed, reversed_expected)
    }
}
