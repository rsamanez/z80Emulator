// CIA chip
use c64::cpu;
use c64::memory;
use c64::vic;
use std::rc::Rc;
use std::cell::RefCell;

pub type CIAShared = Rc<RefCell<CIA>>;

enum TimerState {
    Stop,
    WaitCount,
    LoadStop,
    LoadCount,
    LoadWaitCount,
    Count,
    CountStop
}


// Struct for CIA timer A/B
struct CIATimer {
    state: TimerState, // current state of the timer
    is_ta: bool,       // is this timer A?
    value: u16,        // timer value (TA/TB)
    latch: u16,        // timer latch
    ctrl:  u8,         // control timer (CRA/CRB)
    new_ctrl: u8,
    has_new_ctrl: bool,
    is_cnt_phi2:  bool,     // timer is counting phi2
    irq_next_cycle: bool,   // perform timer interrupt next cycle
    underflow: bool,        // timer underflowed
    cnt_ta_underflow: bool, // timer is counting underflows of Timer A 
}

impl CIATimer {
    pub fn new(is_ta: bool) -> CIATimer {
        CIATimer {
            state: TimerState::Stop,
            is_ta: is_ta,
            value: 0xFFFF,
            latch: 1,
            ctrl:  0,
            new_ctrl: 0,
            has_new_ctrl: false,
            is_cnt_phi2:  false,
            irq_next_cycle:   false,
            underflow:        false,
            cnt_ta_underflow: false,
        }
    }

    
    pub fn reset(&mut self) {
        self.state    = TimerState::Stop;
        self.value    = 0xFFFF;
        self.latch    = 1;
        self.ctrl     = 0;
        self.new_ctrl = 0;
        self.has_new_ctrl     = false;
        self.is_cnt_phi2      = false;
        self.irq_next_cycle   = false;
        self.underflow        = false;
        self.cnt_ta_underflow = false;
    }


    pub fn update(&mut self, cia_icr: &mut u8, ta_underflow: bool) {
        match self.state {
            TimerState::Stop => (),
            TimerState::WaitCount => {
                self.state = TimerState::Count;
            },
            TimerState::LoadStop => {
                self.state = TimerState::Stop;
                self.value = self.latch;
            },
            TimerState::LoadCount => {
                self.state = TimerState::Count;
                self.value = self.latch;
            },
            TimerState::LoadWaitCount => {
                self.state = TimerState::WaitCount;

                if self.value == 1 {
                    self.irq(cia_icr);
                }
                else {
                    self.value = self.latch;
                }
            }
            TimerState::Count => {
                self.count(cia_icr, ta_underflow);
            },
            TimerState::CountStop => {
                self.state = TimerState::Stop;
                self.count(cia_icr, ta_underflow);
            }
        }

        self.idle();
    }


    pub fn idle(&mut self) {
        if self.has_new_ctrl {
            match self.state {
                TimerState::Stop | TimerState::LoadStop => {
                    if (self.new_ctrl & 1) != 0 {
                        if (self.new_ctrl & 0x10) != 0 {
                            self.state = TimerState::LoadWaitCount;
                        }
                        else {
                            self.state = TimerState::WaitCount;
                        }
                    }
                    else {
                        if (self.new_ctrl & 0x10) != 0 {
                            self.state = TimerState::LoadStop;
                        }
                    }
                },
                TimerState::WaitCount | TimerState::LoadCount => {
                    if (self.new_ctrl & 1) != 0 {
                        if (self.new_ctrl & 8) != 0 {
                            self.new_ctrl &= 0xFE;
                            self.state = TimerState::Stop;
                        }
                        else {
                            if (self.new_ctrl & 0x10) != 0 {
                                self.state = TimerState::LoadWaitCount;
                            }
                        }
                    }
                    else {
                        self.state = TimerState::Stop;
                    }
                },
                TimerState::Count => {
                    if (self.new_ctrl & 1) != 0 {
                        if (self.new_ctrl & 0x10) != 0 {
                            self.state = TimerState::LoadWaitCount;
                        }
                    }
                    else {
                        if (self.new_ctrl & 0x10) != 0 {
                            self.state = TimerState::LoadStop;
                        }
                        else {
                            self.state = TimerState::CountStop;
                        }
                    }
                },
                _ => (),
            }

            self.ctrl = self.new_ctrl & 0xEF;
            self.has_new_ctrl = false;
        }
    }

    
    pub fn irq(&mut self, cia_icr: &mut u8) {
        self.value = self.latch;
        self.irq_next_cycle = true;
        *cia_icr |= if self.is_ta { 1 } else { 2 };

        if (self.ctrl & 8) != 0 {
            self.ctrl &= 0xFE;
            self.new_ctrl &= 0xFE;
            self.state = TimerState::LoadStop;
        }
        else {
            self.state = TimerState::LoadCount;
        }
    }


    pub fn count(&mut self, cia_icr: &mut u8, ta_underflow: bool) {
        if self.is_cnt_phi2 || (self.cnt_ta_underflow && ta_underflow) {
            let curr_val = self.value;
            self.value -= 1;
            if (curr_val == 0) || (self.value == 0) {
                match self.state {
                    TimerState::Stop => (),
                    _ => self.irq(cia_icr),
                }

                self.underflow = true;
            }
        }
    }
}


// the actual CIA chip including both timers
pub struct CIA {
    mem_ref: Option<memory::MemShared>,
    cpu_ref: Option<cpu::CPUShared>,
    vic_ref: Option<vic::VICShared>,

    is_cia1: bool,  // is this CIA1 or CIA2 chip?
    
    timer_a: CIATimer,
    timer_b: CIATimer,
    irq_mask: u8,
    icr:  u8,
    pra:  u8,
    prb:  u8,
    ddra: u8,
    ddrb: u8,
    sdr:  u8,
    
    // TOD timer
    tod_halt: bool,
    tod_freq_div: u16,
    tod_hour: u8,
    tod_min:  u8,
    tod_sec:  u8,
    tod_dsec: u8, // deciseconds

    // alarm time
    alarm_hour: u8,
    alarm_min:  u8,
    alarm_sec:  u8,
    alarm_dsec: u8,

    // CIA1 only
    pub key_matrix: [u8; 8],
    pub rev_matrix: [u8; 8],
    pub joystick_1: u8,
    pub joystick_2: u8,
    prev_lp: u8,

    // CIA2 only
    iec_lines: u8,
}

impl CIA {
    pub fn new_shared(is_cia1: bool) -> CIAShared {
        Rc::new(RefCell::new(CIA {
            mem_ref: None,
            cpu_ref: None,
            vic_ref: None,

            is_cia1: is_cia1,
            timer_a: CIATimer::new(true),
            timer_b: CIATimer::new(false),
            irq_mask: 0,
            icr: 0,
            pra: 0,
            prb: 0,
            ddra: 0,
            ddrb: 0,
            sdr: 0,

            tod_halt: false,
            tod_freq_div: 0,
            tod_hour: 0,
            tod_min: 0,
            tod_sec: 0,
            tod_dsec: 0,
            alarm_hour: 0,
            alarm_min: 0,
            alarm_sec: 0,
            alarm_dsec: 0,

            // CIA1 only
            key_matrix: [0xFF; 8],
            rev_matrix: [0xFF; 8],
            joystick_1: 0xFF,
            joystick_2: 0xFF,
            prev_lp: 0x10,

            // CIA2 only
            iec_lines: 0xD0
        }))
    }


    pub fn set_references(&mut self, memref: memory::MemShared, cpuref: cpu::CPUShared, vicref: vic::VICShared) {
        self.mem_ref = Some(memref);
        self.cpu_ref = Some(cpuref);
        self.vic_ref = Some(vicref);
    }


    pub fn reset(&mut self) {
        self.timer_a.reset();
        self.timer_b.reset();
        self.irq_mask = 0;
        self.icr = 0;
        self.pra = 0;
        self.prb = 0;
        self.ddra = 0;
        self.ddrb = 0;
        self.sdr = 0;
        self.tod_halt = false;
        self.tod_freq_div = 0;
        self.tod_hour = 0;
        self.tod_min  = 0;
        self.tod_sec  = 0;
        self.tod_dsec = 0;
        self.alarm_hour = 0;
        self.alarm_min  = 0;
        self.alarm_sec  = 0;
        self.alarm_dsec = 0;

        // CIA1 only
        for i in 0..8 {
            self.key_matrix[i] = 0xFF;
            self.rev_matrix[i] = 0xFF;
        }

        self.joystick_1 = 0xFF;
        self.joystick_2 = 0xFF;
        self.prev_lp = 0x10;

        // CIA2 only
        self.iec_lines = 0xD0;
    }


    pub fn update(&mut self) {
        self.timer_a.update(&mut self.icr, false);
        let ta_underflow = self.timer_a.underflow;
        self.timer_b.update(&mut self.icr, ta_underflow);
    }


    pub fn read_register(&mut self, addr: u16, on_cia_read: &mut cpu::Callback) -> u8 {
        // CIA1 and CIA2 share behavior for certain addresses
        match addr & 0x00FF {
            0x02 => self.ddra,
            0x03 => self.ddrb,
            0x04 =>  self.timer_a.value as u8,
            0x05 => (self.timer_a.value >> 8) as u8,
            0x06 => self.timer_b.value as u8,
            0x07 => (self.timer_b.value >> 8) as u8,
            0x08 => {
                self.tod_halt = false;
                self.tod_dsec
            },
            0x09 => self.tod_sec,
            0x0A => self.tod_min,
            0x0B => {
                self.tod_halt = true;
                self.tod_hour
            },
            0x0C => self.sdr,
            0x0D => {
                let curr_icr = self.icr;
                self.icr = 0;
                *on_cia_read = if self.is_cia1 { cpu::Callback::ClearCIAIrq } else { cpu::Callback::ClearNMI };
                curr_icr
            },
            0x0E => self.timer_a.ctrl,
            0x0F => self.timer_b.ctrl,
            0x10..=0xFF => self.read_register((addr & 0xFF00) + (addr % 0x0010), on_cia_read),
            _ => {
                // CIA1/2 specific read-values for 0x00 and 0x01
                if self.is_cia1 {
                    self.read_cia1_register(addr)
                }
                else {
                    self.read_cia2_register(addr)
                }
            }
        }
    }


    pub fn write_register(&mut self, addr: u16, value: u8, on_cia_write: &mut cpu::Callback) {
        match addr & 0x00FF {
            0x04 => {
                self.timer_a.latch = (self.timer_a.latch & 0xFF00) | value as u16;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x05 => {
                self.timer_a.latch = (self.timer_a.latch & 0x00FF) | ((value as u16) << 8);
                if (self.timer_a.ctrl & 1) == 0 {
                    self.timer_a.value = self.timer_a.latch;
                }
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x06 => {
                self.timer_b.latch = (self.timer_b.latch & 0xFF00) | value as u16;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x07 => {
                self.timer_b.latch = (self.timer_b.latch & 0x00FF) | ((value as u16) << 8);
                if (self.timer_b.ctrl & 1) == 0 {
                    self.timer_b.value = self.timer_b.latch;
                }
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x08 => {
                if (self.timer_b.ctrl & 0x80) != 0 {
                    self.alarm_dsec = value & 0x0F;
                }
                else {
                    self.tod_dsec = value & 0x0F;
                }
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x09 => {
                if (self.timer_b.ctrl & 0x80) != 0 {
                    self.alarm_sec = value & 0x7F;
                }
                else {
                    self.tod_sec = value & 0x7F;
                }
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x0A => {
                if (self.timer_b.ctrl & 0x80) != 0 {
                    self.alarm_min = value & 0x7F;
                }
                else {
                    self.tod_min = value & 0x7F;
                }
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
             0x0B => {
                if (self.timer_b.ctrl & 0x80) != 0 {
                    self.alarm_hour = value & 0x9F;
                }
                else {
                    self.tod_hour = value & 0x9F;
                }
                 as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
             },
            0x0C => {
                self.sdr = value;
                let irq_triggered = self.trigger_irq(8);
                if irq_triggered {
                    *on_cia_write = if self.is_cia1 { cpu::Callback::TriggerCIAIrq } else { cpu::Callback::TriggerNMI };
                }
            },
            0x0D => {
                if (value & 0x80) != 0 {
                    self.irq_mask |= value & 0x7F;
                }
                else {
                    self.irq_mask &= !value;
                }

                if (self.icr & self.irq_mask & 0x1F) != 0 {
                    self.icr |= 0x80;
                    *on_cia_write = if self.is_cia1 { cpu::Callback::TriggerCIAIrq } else { cpu::Callback::TriggerNMI };
                }
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x0E => {
                self.timer_a.has_new_ctrl = true;
                self.timer_a.new_ctrl = value;
                self.timer_a.is_cnt_phi2 = (value & 0x20) == 0;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0x0F => {
                self.timer_b.has_new_ctrl = true;
                self.timer_b.new_ctrl = value;
                self.timer_b.is_cnt_phi2 = (value & 0x60) == 0;
                self.timer_b.cnt_ta_underflow = (value & 0x60) == 0x40;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            _ => {
                if self.is_cia1 {
                    self.write_cia1_register(addr, value, on_cia_write);
                }
                else {
                    self.write_cia2_register(addr, value, on_cia_write);
                }
            }
        }
    }


    pub fn process_irq(&mut self) {
        if self.timer_a.irq_next_cycle {
            if self.trigger_irq(1) {
                if self.is_cia1 {
                    as_mut!(self.cpu_ref).set_cia_irq(true);
                }
                else {
                    as_mut!(self.cpu_ref).set_nmi(true);
                }
            }
            
            self.timer_a.irq_next_cycle = false
        }
        if self.timer_a.irq_next_cycle {
            if self.trigger_irq(2) {
                if self.is_cia1 {
                    as_mut!(self.cpu_ref).set_cia_irq(true);
                }
                else {
                    as_mut!(self.cpu_ref).set_nmi(true);
                }
            }
            
            self.timer_a.irq_next_cycle = false
        }
    }


    pub fn count_tod(&mut self) {
        let mut lo: u8;
        let mut hi: u8;

        if self.tod_freq_div != 0 {
            self.tod_freq_div -= 1;
        }
        else {
            // adjust frequency according to 50/60Hz flag
            if (self.timer_a.ctrl & 0x80) != 0 {
                self.tod_freq_div = 4;
            }
            else {
                self.tod_freq_div = 5;
            }

            self.tod_dsec += 1;
            if self.tod_dsec > 9 {
                self.tod_dsec = 0;

                lo = (self.tod_sec & 0x0F) + 1;
                hi = self.tod_sec >> 4;

                if lo > 9 {
                    lo = 0;
                    hi += 1;
                }

                if hi > 5 {
                    self.tod_sec = 0;

                    lo = (self.tod_min & 0x0F) + 1;
                    hi = self.tod_min >> 4;

                    if lo > 9 {
                        lo = 0;
                        hi += 1;
                    }

                    if hi > 5 {
                        self.tod_min = 0;

                        lo = (self.tod_hour & 0x0F) + 1;
                        hi = self.tod_hour >> 4;

                        if lo > 9 {
                            lo = 0;
                            hi += 1;
                        }

                        self.tod_hour |= (hi << 4) | lo;
                        if (self.tod_hour & 0x1F) > 0x11 {
                            self.tod_hour = self.tod_hour & 0x80 ^ 0x80;
                        }
                    }
                    else {
                        self.tod_min = (hi << 4) | lo;
                    }
                }
                else {
                    self.tod_sec = (hi << 4) | lo;
                }
            }

            // TODO: update memory registers
            // trigger irq if alarm time reached
            if (self.tod_dsec == self.alarm_dsec) &&
               (self.tod_sec  == self.alarm_sec)  &&
               (self.tod_min  == self.alarm_min)  &&
               (self.tod_hour == self.alarm_hour) {
                if self.trigger_irq(4) {
                    if self.is_cia1 {
                        as_mut!(self.cpu_ref).set_cia_irq(true);
                    }
                    else {
                        as_mut!(self.cpu_ref).set_nmi(true);
                    };
                }
            }
        }
    }


    // true - irq triggered; false - not
    pub fn trigger_irq(&mut self, mask: u8) -> bool {
        self.icr |= mask;

        if (self.irq_mask & mask) != 0 {
            self.icr |= 0x80;
            true
        }
        else {
            false
        }
    }


    // *** private functions *** //

    fn read_cia1_register(&mut self, addr: u16) -> u8 {
        match addr {
            0xDC00 => {
                let mut retval = self.pra | !self.ddra;
                let tst = (self.prb | !self.ddrb) & self.joystick_1;

                if tst & 0x01 == 0 { retval &= self.rev_matrix[0]; }
                if tst & 0x02 == 0 { retval &= self.rev_matrix[1]; }
                if tst & 0x04 == 0 { retval &= self.rev_matrix[2]; }
                if tst & 0x08 == 0 { retval &= self.rev_matrix[3]; }
                if tst & 0x10 == 0 { retval &= self.rev_matrix[4]; }
                if tst & 0x20 == 0 { retval &= self.rev_matrix[5]; }
                if tst & 0x40 == 0 { retval &= self.rev_matrix[6]; }
                if tst & 0x80 == 0 { retval &= self.rev_matrix[7]; }
                
                retval & self.joystick_2
            },
            0xDC01 => {
                let mut retval = !self.ddrb;
                let tst = (self.pra | !self.ddra) & self.joystick_2;

                if tst & 0x01 == 0 { retval &= self.key_matrix[0]; }
                if tst & 0x02 == 0 { retval &= self.key_matrix[1]; }
                if tst & 0x04 == 0 { retval &= self.key_matrix[2]; }
                if tst & 0x08 == 0 { retval &= self.key_matrix[3]; }
                if tst & 0x10 == 0 { retval &= self.key_matrix[4]; }
                if tst & 0x20 == 0 { retval &= self.key_matrix[5]; }
                if tst & 0x40 == 0 { retval &= self.key_matrix[6]; }
                if tst & 0x80 == 0 { retval &= self.key_matrix[7]; }

                (retval | (self.prb & self.ddrb)) & self.joystick_1
            },
            0xDC10..=0xDCFF => self.read_cia1_register(0xDC00 + (addr % 0x0010)),
            _ => panic!("Address out of CIA1 memory range: ${:04X}", addr),
        }
    }


    fn write_cia1_register(&mut self, addr: u16, value: u8, on_cia_write: &mut cpu::Callback) {
        match addr {
            0xDC00 => {
                self.pra = value;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0xDC01 => {
                self.prb = value;
                self.check_lp();
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0xDC02 => {
                self.ddra = value;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0xDC03 => {
                self.ddrb = value;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
                self.check_lp();
            },
            0xDC10..=0xDCFF => self.write_cia1_register(0xDC00 + (addr % 0x0010), value, on_cia_write),
            _ => panic!("Address out of CIA1 memory range"),
        }
    }


    fn read_cia2_register(&mut self, addr: u16) -> u8 {
        match addr {
            0xDD00 => {
                // TODO
                (self.pra | !self.ddra) & 0x3f | self.iec_lines
            },
            0xDD01 => self.prb | !self.ddrb,
            0xDD10..=0xDDFF => self.read_cia2_register(0xDD00 + (addr % 0x0010)),
            _ => panic!("Address out of CIA2 memory range ${:04X}", addr),
        }
    }


    fn write_cia2_register(&mut self, addr: u16, value: u8, on_cia_write: &mut cpu::Callback) {
        match addr {
            0xDD00 => {
                // TODO
                self.pra = value;
                as_mut!(self.vic_ref).on_va_change(!(self.pra | !self.ddra) & 3);
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0xDD01 => {
                self.prb = value;
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0xDD02 => {
                self.ddra = value;
                as_mut!(self.vic_ref).on_va_change(!(self.pra | !self.ddra) & 3);
                as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value);
            },
            0xDD03 => { self.ddrb = value; as_ref!(self.mem_ref).get_ram_bank(memory::MemType::Io).write(addr, value); },
            0xDD10..=0xDDFF => self.write_cia2_register(0xDD00 + (addr % 0x0010), value, on_cia_write),
            _ => panic!("Address out of CIA2 memory range"),
        }
    }


    fn check_lp(&mut self) {
        if ((self.prb | !self.ddrb) & 0x10) != self.prev_lp {
            as_mut!(self.vic_ref).trigger_lp_irq();
        }

        self.prev_lp = (self.prb | !self.ddrb) & 0x10;
    }
}
