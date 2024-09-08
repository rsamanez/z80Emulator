// Using minifb to manage window presentation to emulate the CPU Screeen
use minifb::*;
use minifb_fonts::*;

//TEXT RESOLUTION 80X25 USING 6X8 FONT SIZE
const SCREEN_WIDTH: usize = 480;
const SCREEN_HEIGHT: usize = 200;


fn main() {
    fn from_u8_rgb(r: u8, g: u8, b: u8) -> u32 {
        let (r, g, b) = (r as u32, g as u32, b as u32);
        (r << 16) | (g << 8) | b
    }
    let window_width = SCREEN_WIDTH;
    let window_height = SCREEN_HEIGHT;
    let buffer_width = SCREEN_WIDTH;
    let buffer_height = SCREEN_HEIGHT;
    let window_scale = Scale::X2;
 
    
    let mut color = from_u8_rgb(0, 0, 0);
    let mut buffer: Vec<u32> = vec![color; buffer_width * buffer_height];
    
    let mut window = Window::new("Test", window_width, window_height, WindowOptions { scale: window_scale, ..WindowOptions::default() }).unwrap();
    window.set_position(75, 20);
    
    
    color = from_u8_rgb(250, 0, 0);

    let mut text = font6x8::new_renderer(buffer_width, buffer_height, color);
    text.draw_text(&mut buffer, 0, 0, "ABCDEFGHIJKLMNOPQRSTUVWXYZ1234567890");
    text.draw_text(&mut buffer, 0, 8, "12345678901234567890123456789012345678901234567890123456789012345678901234567890");
    text.draw_text(&mut buffer, 0, 16, "AAAAAAAAAAAAAABBBBBBBBBBBBBBBBBBBBCCCCCCCCCCCCCCCCCCCCCDDDDDDDDDDDDDDEEEEEEEEEEE");
    text.set_color(0x00_ff_00);
    text.draw_text(&mut buffer, 10, 180, "Press ESC to exit");
    text.draw_text(&mut buffer, 10, 30, "A");
    text.draw_text(&mut buffer, 16, 30, "B");
    text.draw_text(&mut buffer, 22, 30, "C");
    text.draw_text(&mut buffer, 10, 38, "D");
    text.draw_text(&mut buffer, 16, 38, "E");
    text.draw_text(&mut buffer, 22, 38, "F");
    while window.is_open() && !window.is_key_down(minifb::Key::Escape){
        window.update_with_buffer(&buffer, buffer_width, buffer_height).unwrap();
    }
}
