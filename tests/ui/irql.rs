#![crate_type = "lib"]

// These dummy functions simulate the IRQL manipulation API
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

pub fn multiple_raise_multiple_lower_to_previous_values() {
    KeRaiseIrql(6);
    KeRaiseIrql(7);
    KeRaiseIrql(8);
    KeLowerIrql(7);
    KeLowerIrql(6);
    KeLowerIrql(0);
}
