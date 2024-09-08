// Using minifb to manage window presentation to emulate the CPU Screeen
use minifb::*;


const SCREEN_WIDTH: usize = 640;
const SCREEN_HEIGHT: usize = 480;


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
    let x_buffer_width = 640;
    let x_buffer_height = 480;
    
    let mut color = from_u8_rgb(0, 0, 0);
    let mut buffer: Vec<u32> = vec![color; buffer_width * buffer_height];
    
    let mut window = Window::new("Test", window_width, window_height, WindowOptions { scale: window_scale, ..WindowOptions::default() }).unwrap();
    window.set_position(75, 20);
    
    let mut offset: usize;
    let mut alfa: f32 = 0.0;
    let mut delta: f32 = 0.0;
    let mut rx: f32 = 0.0;
    let mut ry: f32 = 0.0;
    let mut rY:u32 = 0;
    let mut rX:u32 = 0;
    color = from_u8_rgb(250, 0, 0);
    rx = 200.0;//*alfa.sin()+240.0;
    ry = 200.0;//*alfa.sin()+240.0;
    while window.is_open(){
        alfa = alfa+0.01;
        if alfa >= 3.1415*2.0 {
            alfa = 0.0;
            delta = delta+0.5;
        }
        ry = 200.0*(alfa+delta).cos()+240.0;
        rx = 200.0*alfa.sin()+240.0;
        rY = ry as u32;
        rX = rx as u32;
        offset = (rX+rY*640) as usize;
        
        buffer[offset] = color;
        window.update_with_buffer(&buffer, x_buffer_width, x_buffer_height).unwrap();
    }
}
