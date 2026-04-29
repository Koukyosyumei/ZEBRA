use std::fmt;

use crate::interval::{AbstractInterval, MayBeFlag};

pub fn reconstruct_word(row: &[AbstractInterval], base: usize, len: usize) -> AbstractInterval {
    let mut val = AbstractInterval::from_i128(0);
    let mut mul = 1_i128;
    for i in 0..len {
        val = val + row[base + i].clone() * AbstractInterval::from_i128(mul);
        mul *= 256;
    }
    val
}

#[derive(Clone, Debug)]
struct Segment {
    addr: AbstractInterval,
    value: AbstractInterval,
}

pub struct IntervalMemory {
    /// Invariant: segments are pairwise non-overlapping.
    segs: Vec<Segment>,
}

impl fmt::Display for IntervalMemory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let rows: Vec<(String, String)> = self
            .segs
            .iter()
            .map(|s| (s.addr.to_string(), s.value.to_string()))
            .collect();

        let addr_w = rows
            .iter()
            .map(|(a, _)| a.len())
            .chain(std::iter::once("Address Interval".len()))
            .max()
            .unwrap_or(0);

        writeln!(
            f,
            "  {:<addr_w$} | {}",
            "Address Interval", "Value Interval"
        )?;
        writeln!(f, "  {:-<addr_w$}-+-{:-<20}", "", "")?;

        for (addr, value) in rows {
            writeln!(f, "  {:<addr_w$} | {}", addr, value)?;
        }

        write!(f, "\n")
    }
}

impl IntervalMemory {
    pub fn new() -> Self {
        Self { segs: vec![] }
    }

    pub fn write(&mut self, addr: &AbstractInterval, value: &AbstractInterval) {
        let mut new_segs = Vec::new();

        for seg in &self.segs {
            if seg.addr.is_disjoint(addr) {
                new_segs.push(seg.clone());
            } else {
                if seg.addr.lo < addr.lo {
                    new_segs.push(Segment {
                        addr: AbstractInterval {
                            lo: seg.addr.lo,
                            hi: addr.lo - 1,
                        },
                        value: seg.value.clone(),
                    });
                }

                if seg.addr.hi > addr.hi {
                    new_segs.push(Segment {
                        addr: AbstractInterval {
                            lo: addr.hi + 1,
                            hi: seg.addr.hi,
                        },
                        value: seg.value.clone(),
                    });
                }
            }
        }

        new_segs.push(Segment {
            addr: addr.clone(),
            value: value.clone(),
        });

        self.segs = new_segs;
    }

    pub fn check_read(&self, addr: &AbstractInterval, val: &AbstractInterval) -> MayBeFlag {
        let zero = AbstractInterval { lo: 0, hi: 0 };

        let mut all_contained = true;
        let mut all_disjoint = true;

        for seg in &self.segs {
            if seg.addr.is_intersects(addr) {
                if !val.is_contains(&seg.value) {
                    all_contained = false;
                }
                if !seg.value.is_disjoint(val) {
                    all_disjoint = false;
                }
            }
        }

        // Unwritten regions read as 0.
        if !self.covers(addr) {
            if !val.is_contains(&zero) {
                all_contained = false;
            }
            if !zero.is_disjoint(val) {
                all_disjoint = false;
            }
        }

        if all_contained {
            MayBeFlag::True
        } else if all_disjoint {
            MayBeFlag::False
        } else {
            MayBeFlag::MayBe
        }
    }

    fn covers(&self, addr: &AbstractInterval) -> bool {
        let mut covered_lo = addr.lo;

        let mut segs: Vec<_> = self
            .segs
            .iter()
            .filter(|s| s.addr.is_intersects(addr))
            .collect();

        segs.sort_by_key(|s| s.addr.lo);

        for s in segs {
            if s.addr.lo > covered_lo {
                return false;
            }
            covered_lo = covered_lo.max(s.addr.hi + 1);
            if covered_lo > addr.hi {
                return true;
            }
        }

        false
    }
}

pub fn check_memory_consistency(
    ops: &[(AbstractInterval, AbstractInterval, bool)],
) -> (IntervalMemory, MayBeFlag) {
    let mut mem = IntervalMemory::new();
    let mut result = MayBeFlag::True;

    for (addr, val, is_write) in ops {
        if *is_write {
            mem.write(addr, val);
        } else {
            let r = mem.check_read(addr, val);
            result = combine(&result, &r);
            if r == MayBeFlag::False {
                return (mem, MayBeFlag::False);
            }
        }
    }

    (mem, result)
}

fn combine(a: &MayBeFlag, b: &MayBeFlag) -> MayBeFlag {
    match (a, b) {
        (MayBeFlag::False, _) | (_, MayBeFlag::False) => MayBeFlag::False,
        (MayBeFlag::MayBe, _) | (_, MayBeFlag::MayBe) => MayBeFlag::MayBe,
        _ => MayBeFlag::True,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn iv(lo: i128, hi: i128) -> AbstractInterval {
        AbstractInterval { lo, hi }
    }

    // =========================================================
    // write tests
    // =========================================================

    #[test]
    fn write_into_empty_memory() {
        let mut mem = IntervalMemory::new();
        mem.write(&iv(10, 20), &iv(1, 1));

        assert_eq!(mem.segs.len(), 1);
        assert_eq!(mem.segs[0].addr, iv(10, 20));
        assert_eq!(mem.segs[0].value, iv(1, 1));
    }

    #[test]
    fn write_non_overlapping_keeps_both() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(1, 1));
        mem.write(&iv(20, 29), &iv(2, 2));

        assert_eq!(mem.segs.len(), 2);
    }

    #[test]
    fn write_overwrites_middle_and_splits() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 99), &iv(1, 1));
        mem.write(&iv(20, 29), &iv(2, 2));

        assert_eq!(mem.segs.len(), 3);

        assert_eq!(mem.segs[0].addr, iv(0, 19));
        assert_eq!(mem.segs[0].value, iv(1, 1));
        assert_eq!(mem.segs[1].addr, iv(30, 99));
        assert_eq!(mem.segs[1].value, iv(1, 1));
        assert_eq!(mem.segs[2].addr, iv(20, 29));
        assert_eq!(mem.segs[2].value, iv(2, 2));
    }

    #[test]
    fn write_full_overwrite() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 99), &iv(1, 1));
        mem.write(&iv(0, 99), &iv(2, 2));

        assert_eq!(mem.segs.len(), 1);
        assert_eq!(mem.segs[0].value, iv(2, 2));
    }

    #[test]
    fn write_partial_overlap_left() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(10, 30), &iv(1, 1));
        mem.write(&iv(0, 15), &iv(2, 2));

        assert_eq!(mem.segs.len(), 2);

        assert_eq!(mem.segs[0].addr, iv(16, 30));
        assert_eq!(mem.segs[0].value, iv(1, 1));
        assert_eq!(mem.segs[1].addr, iv(0, 15));
        assert_eq!(mem.segs[1].value, iv(2, 2));
    }

    // =========================================================
    // check_read tests
    // =========================================================

    #[test]
    fn read_from_empty_memory_is_true() {
        let mem = IntervalMemory::new();

        let r = mem.check_read(&iv(0, 10), &iv(0, 0));
        assert_eq!(r, MayBeFlag::True);
    }

    fn read_from_empty_memory_is_false() {
        let mem = IntervalMemory::new();

        let r = mem.check_read(&iv(0, 10), &iv(5, 5));
        assert_eq!(r, MayBeFlag::False);
    }

    fn read_from_empty_memory_is_maybe() {
        let mem = IntervalMemory::new();

        let r = mem.check_read(&iv(0, 10), &iv(0, 5));
        assert_eq!(r, MayBeFlag::MayBe);
    }

    #[test]
    fn read_exact_match_true() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(7, 7));

        let r = mem.check_read(&iv(0, 9), &iv(7, 7));
        assert_eq!(r, MayBeFlag::True);
    }

    #[test]
    fn read_value_superset_true() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(5, 10));

        let r = mem.check_read(&iv(0, 9), &iv(0, 20));
        assert_eq!(r, MayBeFlag::True);
    }

    #[test]
    fn read_disjoint_false() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(10, 20));

        let r = mem.check_read(&iv(0, 9), &iv(0, 5));
        assert_eq!(r, MayBeFlag::False);
    }

    #[test]
    fn read_partial_overlap_maybe() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(10, 20));

        let r = mem.check_read(&iv(0, 9), &iv(15, 30));
        assert_eq!(r, MayBeFlag::MayBe);
    }

    #[test]
    fn read_multiple_segments_all_valid_true() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(1, 1));
        mem.write(&iv(10, 19), &iv(2, 2));

        let r = mem.check_read(&iv(0, 19), &iv(0, 5));
        assert_eq!(r, MayBeFlag::True);
    }

    #[test]
    fn read_multiple_segments_some_invalid_maybe() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(1, 1));
        mem.write(&iv(10, 19), &iv(100, 100));

        let r = mem.check_read(&iv(0, 19), &iv(0, 10));
        assert_eq!(r, MayBeFlag::MayBe);
    }

    #[test]
    fn read_multiple_segments_all_invalid_false() {
        let mut mem = IntervalMemory::new();

        mem.write(&iv(0, 9), &iv(100, 100));
        mem.write(&iv(10, 19), &iv(200, 200));

        let r = mem.check_read(&iv(0, 19), &iv(0, 50));
        assert_eq!(r, MayBeFlag::False);
    }

    // =========================================================
    // check_memory_consistency tests
    // =========================================================

    #[test]
    fn simple_write_then_read_true() {
        let ops = vec![(iv(0, 9), iv(5, 5), true), (iv(0, 9), iv(5, 5), false)];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::True);
    }

    #[test]
    fn read_wrong_value_false() {
        let ops = vec![(iv(0, 9), iv(5, 5), true), (iv(0, 9), iv(6, 6), false)];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::False);
    }

    #[test]
    fn read_possible_value_maybe() {
        let ops = vec![(iv(0, 9), iv(5, 10), true), (iv(0, 9), iv(8, 20), false)];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::MayBe);
    }

    #[test]
    fn multiple_writes_last_one_counts() {
        let ops = vec![
            (iv(0, 9), iv(1, 1), true),
            (iv(0, 9), iv(2, 2), true),
            (iv(0, 9), iv(2, 2), false),
        ];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::True);
    }

    #[test]
    fn read_uninitialized_maybe() {
        let ops = vec![(iv(0, 9), iv(5, 5), false)];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::False);
    }

    #[test]
    fn mixed_reads_true_overall() {
        let ops = vec![
            (iv(0, 9), iv(1, 1), true),
            (iv(0, 9), iv(1, 1), false),
            (iv(0, 9), iv(1, 1), false),
        ];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::True);
    }

    #[test]
    fn mixed_reads_with_maybe() {
        let ops = vec![
            (iv(0, 9), iv(1, 10), true),
            (iv(0, 9), iv(5, 5), false),
            (iv(0, 9), iv(20, 30), false), // impossible
        ];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::False);
    }

    #[test]
    fn complex_sequence() {
        let ops = vec![
            (iv(0, 49), iv(1, 1), true),
            (iv(50, 99), iv(2, 2), true),
            (iv(0, 99), iv(0, 5), false),  // both valid
            (iv(25, 75), iv(1, 2), false), // both valid
        ];

        assert_eq!(check_memory_consistency(&ops).1, MayBeFlag::True);
    }
}
