#![crate_type = "lib"]

#[no_mangle]
pub extern "C" fn KeRaiseIrql(level: u32) {}

#[no_mangle]
pub extern "C" fn KeLowerIrql(level: u32) {}

#[klint::irql(raise = 1)]
fn raise_to_apc() {
    KeRaiseIrql(1);
}

#[klint::irql(raise = 2)]
fn raise_to_dispatch() {
    KeRaiseIrql(2);
}

#[klint::irql(require = 1)]
fn f1() {}

#[klint::irql(require = 2)]
fn f2() {}

fn f3(do_high: bool) {
    if do_high {
        raise_to_dispatch();
        f2();
        KeLowerIrql(1);
    } else {
        f1();
    }
}

fn f4(handler: fn()) {
    raise_to_apc();
    handler();
    KeLowerIrql(0);
}

fn f5(run_fast: bool, alt_path: bool) {
    if run_fast {
        raise_to_apc();
        if alt_path {
            f3(false);
        } else {
            f3(true);
        }
        KeLowerIrql(0);
    } else {
        f1();
    }
}

fn wrapper(callee: fn(bool), flag: bool) {
    callee(flag);
}

pub fn entry_point() {
    f5(true, false);
    f4(f1);
    f4(f2);

    wrapper(|flag| f5(flag, true), false);
}
