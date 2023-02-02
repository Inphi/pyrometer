use ethers_core::types::Address;
use crate::context::ContextNode;
use std::collections::BTreeMap;
use crate::range::elem_ty::Dynamic;
use crate::range::elem::RangeOp;
use crate::range::elem_ty::RangeExpr;
use crate::range::elem_ty::DynSide;
use crate::context::ContextVarNode;
use ethers_core::types::I256;
use ethers_core::types::U256;
use crate::range::elem_ty::RangeConcrete;
use crate::Builtin;
use crate::range::elem_ty::Elem;
use crate::range::elem::RangeElem;
use crate::Concrete;
use crate::analyzer::AnalyzerLike;

use solang_parser::pt::Loc;

pub mod elem;
pub mod elem_ty;
pub mod range_ops;
pub mod range_string;

#[derive(Clone, Debug, PartialEq, Eq, Ord, PartialOrd, Hash)]
pub struct SolcRange {
    pub min: Elem<Concrete>,
    pub max: Elem<Concrete>,
}

impl From<bool> for SolcRange {
    fn from(b: bool) -> Self {
        let val = Elem::Concrete(RangeConcrete { val: Concrete::Bool(b), loc: Loc::Implicit });
        Self {
            min: val.clone(),
            max: val,
        }
    }
}


impl SolcRange {
    pub fn default_bool() -> Self {
        let min = Elem::Concrete(RangeConcrete { val: Concrete::Bool(false), loc: Loc::Implicit });
        let max = Elem::Concrete(RangeConcrete { val: Concrete::Bool(true), loc: Loc::Implicit });
        Self {
            min,
            max,
        }
    }
    pub fn from(c: Concrete) -> Option<Self> {
        match c {
            c @ Concrete::Uint(_, _)
            | c @ Concrete::Int(_, _)
            | c @ Concrete::Bool(_)
            | c @ Concrete::Address(_)
            | c @ Concrete::Bytes(_, _) => {
                Some(SolcRange {
                    min: Elem::Concrete(RangeConcrete { val: c.clone(), loc: Loc::Implicit }),
                    max: Elem::Concrete(RangeConcrete { val: c, loc: Loc::Implicit }),
                })
            }
            e => { println!("from: {:?}", e); None}
        }
    }
    pub fn try_from_builtin(builtin: &Builtin) -> Option<Self> {
        match builtin {
            Builtin::Uint(size) => {
                if *size == 256 {
                    Some(SolcRange {
                        min: Elem::Concrete(RangeConcrete { val: Concrete::Uint(*size, 0.into()), loc: Loc::Implicit }),
                        max: Elem::Concrete(RangeConcrete { val: Concrete::Uint(*size, U256::MAX), loc: Loc::Implicit }),
                    })
                } else {
                    Some(SolcRange {
                        min: Elem::Concrete(RangeConcrete { val: Concrete::Uint(*size, 0.into()), loc: Loc::Implicit }),
                        max: Elem::Concrete(RangeConcrete {
                            val: Concrete::Uint(*size, U256::from(2).pow(U256::from(*size)) - 1),
                            loc: Loc::Implicit,
                        }),
                    })
                }
            }
            Builtin::Int(size) => {
                if *size == 256 {
                    Some(SolcRange {
                        min: Elem::Concrete(RangeConcrete { val: Concrete::Int(*size, I256::MIN), loc: Loc::Implicit }),
                        max: Elem::Concrete(RangeConcrete { val: Concrete::Int(*size, I256::MAX), loc: Loc::Implicit }),
                    })
                } else {
                    let max: I256 =
                        I256::from_raw(U256::from(1u8) << U256::from(size - 1)) - 1.into();
                    let min = max * I256::from(-1i32);
                    Some(SolcRange {
                        min: Elem::Concrete(RangeConcrete { val: Concrete::Int(*size, min), loc: Loc::Implicit }),
                        max: Elem::Concrete(RangeConcrete {
                            val: Concrete::Int(*size, max),
                            loc: Loc::Implicit,
                        }),
                    })
                }
            }
            Builtin::Bool => {
                Some(SolcRange {
                    min: Elem::Concrete(RangeConcrete { val: Concrete::Bool(false), loc: Loc::Implicit }),
                    max: Elem::Concrete(RangeConcrete {
                        val: Concrete::Bool(true),
                        loc: Loc::Implicit,
                    }),
                })
            },
            Builtin::Address => {
                Some(SolcRange {
                    min: Elem::Concrete(RangeConcrete { val: Concrete::Address(Address::from_slice(&[0x00; 20])), loc: Loc::Implicit }),
                    max: Elem::Concrete(RangeConcrete {
                        val: Concrete::Address(Address::from_slice(&[0xff; 20])),
                        loc: Loc::Implicit,
                    }),
                })
            },
            _ => None,
        }
    }

    pub fn lte_dyn(self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min,
            max: self
                .max
                .min(Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc))),
        }
    }

    pub fn gte_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self
                .min
                .max(Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc))),
            max: self.max,
        }
    }

    pub fn lt_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min,
            max: self
                .max
                .min(Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)) - Elem::Concrete(RangeConcrete { val: U256::from(1).into(), loc: Loc::Implicit } )),
        }
    }

    pub fn gt_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self
                .min
                .max(Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)) + Elem::Concrete(RangeConcrete { val: U256::from(1).into(), loc: Loc::Implicit } )),
            max: self.max,
        }
    }

    pub fn dyn_fn_from_op(
        op: RangeOp,
    ) -> (
        &'static dyn Fn(SolcRange, ContextVarNode, (DynSide, DynSide), Loc) -> SolcRange,
        (DynSide, DynSide),
    ) {
        match op {
            RangeOp::Add => (
                &Self::add_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Sub => (
                &Self::sub_dyn,
                (DynSide::Max, DynSide::Min),
            ),
            RangeOp::Mul => (
                &Self::mul_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Div => (
                &Self::div_dyn,
                (DynSide::Max, DynSide::Min),
            ),
            RangeOp::Shr => (
                &Self::shr_dyn,
                (DynSide::Max, DynSide::Min),
            ),
            RangeOp::Shl => (
                &Self::shl_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Mod => (
                &Self::mod_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Min => (
                &Self::min_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Max => (
                &Self::max_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Lt => (
                &Self::lt_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Lte => (
                &Self::lte_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Gt => (
                &Self::gt_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Gte => (
                &Self::gte_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Eq => (
                &Self::eq_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            RangeOp::Neq => (
                &Self::neq_dyn,
                (DynSide::Min, DynSide::Max),
            ),
            e => unreachable!("Comparator operations shouldn't exist in a range: {:?}", e),
        }
    }

    pub fn add_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min + Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)),
            max: self.max + Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)),
        }
    }

    pub fn sub_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min - Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)),
            max: self.max - Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)),
        }
    }

    pub fn mul_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min * Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)),
            max: self.max * Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)),
        }
    }

    pub fn div_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min / Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)),
            max: self.max / Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)),
        }
    }

    pub fn shl_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min << Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)),
            max: self.max << Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)),
        }
    }

    pub fn shr_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min >> Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)),
            max: self.max >> Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)),
        }
    }

    pub fn mod_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self.min.clone().min(self.min % Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc))),
            max: Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)).min(self.max % Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc))),
        }
    }

    pub fn min_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self
                .min
                .min(Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc))),
            max: self
                .max
                .min(Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc))),
        }
    }

    pub fn max_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        Self {
            min: self
                .min
                .max(Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc))),
            max: self
                .max
                .max(Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc))),
        }
    }

    pub fn eq_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        let min = self
                .min
                .max(Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)));
        let max = self
                .max
                .min(Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)));
        Self {
            min: min.clone().max(max.clone()),
            max: min.max(max),
        }
    }

    pub fn neq_dyn(
        self,
        other: ContextVarNode,
        range_sides: (DynSide, DynSide),
        loc: Loc,
    ) -> Self {
        let min = self
                .min
                .neq(Elem::Dynamic(Dynamic::new(other.into(), range_sides.0, loc)));
        let max = self
                .max
                .neq(Elem::Dynamic(Dynamic::new(other.into(), range_sides.1, loc)));
        Self {
            min: min.clone().max(max.clone()),
            max: min.max(max),
        }
    }
}


impl Range<Concrete> for SolcRange {
    type ElemTy = Elem<Concrete>;
    fn range_min(&self) -> Self::ElemTy {
        self.min.clone()
    }
    fn range_max(&self) -> Self::ElemTy {
        self.max.clone()
    }
    fn set_range_min(&mut self, new: Self::ElemTy) {
        self.min = new;
    }
    fn set_range_max(&mut self, new: Self::ElemTy) {
        self.max = new;
    }
}

pub trait Range<T> {
    type ElemTy: RangeElem<T> + Clone;
    fn range_min(&self) -> Self::ElemTy;
    fn range_max(&self) -> Self::ElemTy;
    fn set_range_min(&mut self, new: Self::ElemTy);
    fn set_range_max(&mut self, new: Self::ElemTy);
    fn dependent_on(&self) -> Vec<ContextVarNode> {
        let mut deps = self.range_min().dependent_on();
        deps.extend(self.range_max().dependent_on());
        deps
    }

    fn update_deps(&mut self, ctx: ContextNode, analyzer: &impl AnalyzerLike) {
        let deps = self.dependent_on();
        let mapping: BTreeMap<ContextVarNode, ContextVarNode> = deps.into_iter().map(|dep| {
            (dep, dep.latest_version_in_ctx(ctx, analyzer))
        }).collect();

        let mut min = self.range_min().clone();
        let mut max = self.range_max().clone();
        min.update_deps(&mapping);
        max.update_deps(&mapping);
        self.set_range_min(min);
        self.set_range_max(max);
    }
}

pub trait RangeEval<E, T: RangeElem<E>>: Range<E, ElemTy = T> {
    fn sat(&self, analyzer: &impl AnalyzerLike) -> bool;
    fn unsat(&self, analyzer: &impl AnalyzerLike) -> bool {
        !self.sat(analyzer)
    }
    fn contains(&self, other: &Self, analyzer: &impl AnalyzerLike) -> bool;
}

impl RangeEval<Concrete, Elem<Concrete>> for SolcRange {
    fn sat(&self, analyzer: &impl AnalyzerLike) -> bool {
        match self
            .range_min()
            .eval(analyzer)
            .range_ord(&self.range_max().eval(analyzer))
        {
            None | Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal) => true,
            _ => false,
        }
    }

    fn contains(&self, other: &Self, analyzer: &impl AnalyzerLike) -> bool {
        let min_contains = match self
            .range_min()
            .eval(analyzer)
            .range_ord(&other.range_min().eval(analyzer))
        {
            Some(std::cmp::Ordering::Less) | Some(std::cmp::Ordering::Equal) => true,
            _ => false,
        };

        let max_contains = match self
            .range_max()
            .eval(analyzer)
            .range_ord(&other.range_max().eval(analyzer))
        {
            Some(std::cmp::Ordering::Greater) | Some(std::cmp::Ordering::Equal) => true,
            _ => false,
        };

        min_contains && max_contains
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn it_works() {

    }
}