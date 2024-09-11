extern crate minifb;

pub mod cpu;
pub mod memory;

mod cia;
mod clock;
mod io;

use minifb::*;

pub const SCREEN_WIDTH:  usize = 384; // extend 20 pixels left and right for the borders
pub const SCREEN_HEIGHT: usize = 272; // extend 36 pixels top and down for the borders

// PAL clock frequency in Hz
const CLOCK_FREQ: f64 = 1.5 * 985248.0;

pub struct ZPC {
  pub main_window: minifb::Window,
  //pub file_to_load: String,
  //pub crt_to_load: String,
  memory: memory::MemShared,
  io:     io::IO,
  clock:  clock::Clock,
  cpu:  cpu::CPUShared,
  cia1: cia::CIAShared,
  powered_on: bool,
  boot_complete: bool,
  cycle_count: u32,
}

impl ZPC {
  pub fn new(window_scale: Scale) -> ZPC {
      let memory = memory::Memory::new_shared();
      let cia1   = cia::CIA::new_shared(true);
      let cpu    = cpu::CPU::new_shared();

      let mut zpc = ZPC {
          main_window: Window::new("Z80 Emulator", SCREEN_WIDTH, SCREEN_HEIGHT, WindowOptions { scale: window_scale, ..Default::default() }).unwrap(),
          //file_to_load: String::from(prg_to_load),
          //crt_to_load: String::from(crt_to_load),
          memory: memory.clone(), // shared system memory (RAM, ROM, IO registers)
          io:     io::IO::new(),
          clock:  clock::Clock::new(CLOCK_FREQ),
          cpu:  cpu.clone(),
          cia1: cia1.clone(),
          powered_on: false,
          boot_complete: false,
          cycle_count: 0,
      };

      zpc.main_window.set_position(75, 20);

      // cyclic dependencies are not possible in Rust (yet?), so we have
      // to resort to setting references manually
      zpc.cia1.borrow_mut().set_references(memory.clone(), cpu.clone(), vic.clone());
      zpc.cpu.borrow_mut().set_references(memory.clone(), cia1.clone());

      drop(memory);
      drop(cia1);
      drop(cpu);
     
      zpc
  }


  pub fn reset(&mut self) {
      self.memory.borrow_mut().reset();
      self.cpu.borrow_mut().reset();
      self.cia1.borrow_mut().reset();
  }


  pub fn run(&mut self) {
      // attempt to load a program supplied with command line
      if !self.powered_on {
          // $FCE2 is the power-on reset routine, which searches for and starts
          // a cartridge amongst other things. The cartridge must be loaded here
          self.powered_on = self.cpu.borrow_mut().pc == 0xFCE2;
          /*
          if self.powered_on {
              let crt_file = &self.crt_to_load.to_owned()[..];
              if crt_file.len() > 0 {
                  let crt = crt::Crt::from_filename(crt_file).unwrap();
                  println!("{:?}", crt);
                  crt.load_into_memory(self.memory.borrow_mut());
              }
          }
          */
      }

      if !self.boot_complete {
          // $A480 is the BASIC warm start sequence - safe to assume we can load a cmdline program now
          self.boot_complete = self.cpu.borrow_mut().pc == 0xA480;
          /*
          if self.boot_complete {
              let prg_file = &self.file_to_load.to_owned()[..];

              if prg_file.len() > 0 {
                  self.boot_complete = true; self.load_prg(prg_file);
              }
          }
          */
      }

      // main C64 update - use the clock to time all the operations
      if self.clock.tick() {
          let mut should_trigger_vblank = false;

          self.cia1.borrow_mut().process_irq();
          self.cia1.borrow_mut().update();

          self.cpu.borrow_mut().update(self.cycle_count);

          // redraw the screen and process input on VBlank
          if should_trigger_vblank {
              //let _ = self.main_window.update_with_buffer(&self.vic.borrow_mut().window_buffer, SCREEN_WIDTH, SCREEN_HEIGHT);
              self.io.update(&self.main_window, &mut self.cia1);
              self.cia1.borrow_mut().count_tod();

              if self.io.check_restore_key(&self.main_window) {
                  self.cpu.borrow_mut().set_nmi(true);
              }
          }

          // process special keys: console ASM output and reset switch
          /*
          if self.main_window.is_key_pressed(Key::F11, KeyRepeat::No) {
              let di = self.cpu.borrow_mut().debug_instr;
              self.cpu.borrow_mut().debug_instr = !di;
          }
          */

          if self.main_window.is_key_pressed(Key::F12, KeyRepeat::No) {
              self.reset();
          }

          self.cycle_count += 1;
      }

  }


  // *** private functions *** //
  /* 
  // load a *.prg file
  fn load_prg(&mut self, filename: &str) {
      let prg_data = utils::open_file(filename, 0);
      let start_address: u16 = ((prg_data[1] as u16) << 8) | (prg_data[0] as u16);
      println!("Loading {} to start location at ${:04x} ({})", filename, start_address, start_address);

      for i in 2..(prg_data.len()) {
          self.memory.borrow_mut().write_byte(start_address + (i as u16) - 2, prg_data[i]);
      }
  }
  */
}