#![crate_type = "lib"]

#[no_mangle]
pub extern "C" fn KeRaiseIrql(level: u32) {
    let _ = level;
}

#[no_mangle]
pub extern "C" fn KeLowerIrql(level: u32) {
    let _ = level;
}

pub fn raise_then_lower_to_previous_value() {
    KeRaiseIrql(1);
    KeLowerIrql(0);
}

pub fn raise_then_lower_to_wrong_value() {
    KeRaiseIrql(2);
    KeLowerIrql(1);
}

pub fn lower_no_raise() {
    KeLowerIrql(0);
}

#[klint::irql(raise = 3)]
pub fn raise_no_lower_with_annotation_same_as_raise() {
    KeRaiseIrql(3);
}

#[klint::irql(raise = 3)]
pub fn raise_no_lower_with_annotation_different_from_raise() {
    KeRaiseIrql(4);
}

pub fn raise_no_lower_without_annotation() {
    KeRaiseIrql(5);
}

pub fn nested_changes_to_previous_values() {
    KeRaiseIrql(6);
    {
        KeRaiseIrql(7);
        {
            KeRaiseIrql(8);
            KeLowerIrql(7);
        }
        KeLowerIrql(6);
    }
    KeLowerIrql(0);
}

pub fn nested_changes_to_different_values() {
    KeRaiseIrql(6);
    {
        KeRaiseIrql(7);
        {
            KeRaiseIrql(8);
            KeLowerIrql(3);
        }
        KeLowerIrql(2);
    }
    KeLowerIrql(1);
}

pub fn conditional_branch_join_at_same_level(cond: bool) {
    if cond {
        KeRaiseIrql(9);
        KeLowerIrql(0);
    } else {
        KeRaiseIrql(10);
        KeLowerIrql(0);
    }
}

pub fn conditional_branch_join_at_different_levels(cond: bool) {
    if cond {
        KeRaiseIrql(11);
        KeLowerIrql(0);
    } else {
        KeRaiseIrql(12);
    }
}

pub fn loop_change_with_final_lower() {
    for _ in 0..5 {
        KeRaiseIrql(2);
        KeLowerIrql(0);
    }
    KeLowerIrql(0);
}
