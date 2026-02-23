use crate::interval::{AbstractInterval, MayBeFlag};

#[derive(Clone, Debug)]
struct Segment {
    addr: AbstractInterval,
    value: AbstractInterval,
}

pub struct IntervalMemory {
    segs: Vec<Segment>, // 常に non-overlapping
}

impl IntervalMemory {
    pub fn new() -> Self {
        Self { segs: vec![] }
    }

    // ---- WRITE ----
    pub fn write(&mut self, addr: &AbstractInterval, value: &AbstractInterval) {
        let mut new_segs = Vec::new();

        for seg in &self.segs {
            if seg.addr.is_disjoint(addr) {
                new_segs.push(seg.clone());
            } else {
                // 左側残り
                if seg.addr.lo < addr.lo {
                    new_segs.push(Segment {
                        addr: AbstractInterval {
                            lo: seg.addr.lo,
                            hi: addr.lo - 1,
                        },
                        value: seg.value.clone(),
                    });
                }

                // 右側残り
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

        // 新しい書き込み
        new_segs.push(Segment {
            addr: addr.clone(),
            value: value.clone(),
        });

        self.segs = new_segs;
    }

    // ---- READ CHECK ----
    pub fn check_read(&self, addr: &AbstractInterval, val: &AbstractInterval) -> MayBeFlag {
        let mut any_overlap = false;
        let mut all_contained = true;
        let mut all_disjoint = true;

        for seg in &self.segs {
            if seg.addr.is_intersects(addr) {
                any_overlap = true;

                if !val.is_contains(&seg.value) {
                    all_contained = false;
                }

                if !seg.value.is_disjoint(val) {
                    all_disjoint = false;
                }
            }
        }

        if !any_overlap {
            return MayBeFlag::MayBe; // 初期値不明
        }

        if all_contained {
            MayBeFlag::True
        } else if all_disjoint {
            MayBeFlag::False
        } else {
            MayBeFlag::MayBe
        }
    }
}

pub fn check_memory_consistency(ops: &[(AbstractInterval, AbstractInterval, bool)]) -> MayBeFlag {
    let mut mem = IntervalMemory::new();
    let mut result = MayBeFlag::True;

    for (addr, val, is_write) in ops {
        if *is_write {
            mem.write(addr, val);
        } else {
            let r = mem.check_read(addr, val);
            result = combine(&result, &r);
            if r == MayBeFlag::False {
                return MayBeFlag::False;
            }
        }
    }

    result
}

fn combine(a: &MayBeFlag, b: &MayBeFlag) -> MayBeFlag {
    match (a, b) {
        (MayBeFlag::False, _) | (_, MayBeFlag::False) => MayBeFlag::False,
        (MayBeFlag::MayBe, _) | (_, MayBeFlag::MayBe) => MayBeFlag::MayBe,
        _ => MayBeFlag::True,
    }
}
