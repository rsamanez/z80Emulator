use fltk::{ app, button, window, prelude::*};

fn main() {
    let a = app::App::default();
    let mut win = window::Window::default().with_size(400,300).with_label("Hola Mundo...");
    let mut btn = button::Button::default().with_size(80,30).center_of_parent().with_label("Cleck here");
    win.end();
    win.show();
    btn.set_callback(move |b| {
        b.set_label("Clecked.");
        win.set_label("Nuevo Window Title");
    });
    a.run().unwrap();
}
