mod zpc;

use minifb::*;

fn main() {
    let window_scale = Scale::X2;
    let mut zpc = zpc::ZPC::new(window_scale);
    zpc.reset();

    // main update loop
    while zpc.main_window.is_open() {
        zpc.run();
    }
}
