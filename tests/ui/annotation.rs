// Copyright Gary Guo.
//
// SPDX-License-Identifier: MIT OR Apache-2.0

#![crate_type = "lib"]

#[klint::preempt_count]
fn t1() {}

#[klint::preempt_count()]
fn t2() {}

#[klint::preempt_count(adjust = )]
fn t3() {}

#[klint::preempt_count(expect = )]
fn t4() {}

#[klint::preempt_count(expect = ..)]
fn t5() {}

#[klint::preempt_count(unchecked)]
fn t6() {}

#[klint::irql]
fn t7() {}

#[klint::irql()]
fn t8() {}

#[klint::irql(require)]
fn t9() {}

#[klint::irql(require = )]
fn t10() {}

#[klint::irql(require = 1..)]
fn t11() {}

#[klint::irql(require = 1..=5)]
fn t12() {}

#[klint::irql(require = ..5)]
fn t13() {}

#[klint::irql(require = -1)]
fn t14() {}

#[klint::irql(require = 0..32)]
fn t15() {}

#[klint::irql(require = 5..1)]
fn t16() {}
