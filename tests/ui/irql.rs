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

// A correct use case: raise then lower with matching value
pub fn correct_usage() {
    KeRaiseIrql(1);
    KeLowerIrql(0);
}

// A mismatched use case: raise with one, lower with another
pub fn incorrect_usage() {
    KeRaiseIrql(2);
    KeLowerIrql(1);
}

// A missing-lower case: raise but no lowering
pub fn missing_lower() {
    KeRaiseIrql(3);
}

// A nested IRQL use case
pub fn nested_irqls() {
    KeRaiseIrql(4);
    KeRaiseIrql(5);
    KeLowerIrql(4);
    KeLowerIrql(0);
}

// An incorrect nested IRQL (wrong order)
pub fn nested_wrong_order() {
    KeRaiseIrql(6);
    KeRaiseIrql(7);
    KeLowerIrql(5);
}
