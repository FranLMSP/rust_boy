use std::rc::Rc;
use std::cell::RefCell;
use std::ops::RangeInclusive;
use crate::utils::{
    get_bit,
    BitIndex,
    join_bytes
};
use crate::rom::ROM;
use crate::ppu::{
    PPU,
    LCD_STATUS_ADDRESS,
    LCD_CONTROL_ADDRESS,
    LCD_Y_ADDRESS,
    DMA_ADDRESS,
};
use crate::cpu::{Interrupt};
use crate::timer::{Timer, TIMER_DIVIDER_REGISTER_ADDRESS};
use crate::joypad::{Joypad, JOYPAD_ADDRESS};

pub const BANK_ZERO: RangeInclusive<u16>                 = 0x0000..=0x3FFF;
pub const BANK_SWITCHABLE: RangeInclusive<u16>           = 0x4000..=0x7FFF;
pub const VIDEO_RAM: RangeInclusive<u16>                 = 0x8000..=0x9FFF;
pub const EXTERNAL_RAM: RangeInclusive<u16>              = 0xA000..=0xBFFF;
pub const WORK_RAM_1: RangeInclusive<u16>                = 0xC000..=0xCFFF;
pub const WORK_RAM_2: RangeInclusive<u16>                = 0xD000..=0xDFFF;
pub const ECHO_RAM: RangeInclusive<u16>                  = 0xE000..=0xFDFF;
pub const SPRITE_ATTRIBUTE_TABLE: RangeInclusive<u16>    = 0xFE00..=0xFE9F;
pub const NOT_USABLE: RangeInclusive<u16>                = 0xFEA0..=0xFEFF;
pub const IO_REGISTERS: RangeInclusive<u16>              = 0xFF00..=0xFF7F;
pub const HIGH_RAM: RangeInclusive<u16>                  = 0xFF80..=0xFFFE;
pub const INTERRUPT_ENABLE_REGISTER: RangeInclusive<u16> = 0xFFFF..=0xFFFF;
pub const INTERRUPT_ENABLE_ADDRESS: u16 = 0xFFFF;
pub const INTERRUPT_FLAG_ADDRESS: u16 = 0xFF0F;

pub struct Bus {
    game_rom: ROM,
    data: [u8; 0x10000],
    ppu: Rc<RefCell<PPU>>,
    joypad: Rc<RefCell<Joypad>>,
    timer: Rc<RefCell<Timer>>,
}

impl Bus {
    pub fn new(ppu: Rc<RefCell<PPU>>, joypad: Rc<RefCell<Joypad>>, timer: Rc<RefCell<Timer>>) -> Self {
        let args: Vec<String> = std::env::args().collect();
        if args.len() < 2 {
            println!("Please, specify a ROM file");
            std::process::exit(1);
        }
        let game_rom = match ROM::load_file(&args[1]) {
            Ok(rom) => rom,
            Err(err) => {
                println!("Could not read ROM: {}", err);
                std::process::exit(1);
            },
        };
        let mut data = [0x00; 0x10000];
        // Hardware registers after the bootrom
        data[0xFF00] = 0xCF;
        data[0xFF01] = 0x00;
        data[0xFF02] = 0x7E;
        data[0xFF04] = 0x18;
        data[0xFF05] = 0x00;
        data[0xFF06] = 0x00;
        data[0xFF07] = 0xF8;
        data[0xFF0F] = 0xE1;

        data[0xFF40] = 0x91;
        data[0xFF41] = 0x81;
        data[0xFF42] = 0x00;
        data[0xFF43] = 0x00;
        data[0xFF44] = 0x91;
        data[0xFF45] = 0x00;
        data[0xFF46] = 0xFF;
        data[0xFF47] = 0xFC;

        data[0xFF4A] = 0x00;
        data[0xFF4B] = 0x00;
        data[0xFFFF] = 0x00;

        Self {
            data,
            game_rom,
            ppu,
            joypad,
            timer,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        if BANK_ZERO.contains(&address) || BANK_SWITCHABLE.contains(&address)  || EXTERNAL_RAM.contains(&address) {
            return self.game_rom.read(address);
        } else if address == INTERRUPT_ENABLE_ADDRESS || address == INTERRUPT_FLAG_ADDRESS {
            return 0b11100000 | self.data[address as usize];
        } else if VIDEO_RAM.contains(&address) {
            return self.ppu.borrow().read_vram(address);
        } else if SPRITE_ATTRIBUTE_TABLE.contains(&address) {
            return self.ppu.borrow().read_oam(address);
        } else if address == JOYPAD_ADDRESS {
            return self.joypad.borrow().read(self.data[address as usize]);
        }  else if address == TIMER_DIVIDER_REGISTER_ADDRESS {
            return self.timer.borrow().read_divider();
        }
        self.data[address as usize]
    }

    pub fn read_16bit(&self, address: u16) -> u16 {
        join_bytes(self.read(address.wrapping_add(1)), self.read(address))
    }

    pub fn write(&mut self, address: u16, data: u8) {
        if address == 0xFF01 {
            // print!("{}", data as char); 
        }

        if BANK_ZERO.contains(&address) || BANK_SWITCHABLE.contains(&address) || EXTERNAL_RAM.contains(&address) {
            self.game_rom.write(address, data);
        } else if WORK_RAM_1.contains(&address) || WORK_RAM_2.contains(&address) {
            self.data[address as usize] = data;
            // Copy to the ECHO RAM
            if address <= 0xDDFF {
                self.data[(ECHO_RAM.min().unwrap() + (address - WORK_RAM_1.min().unwrap())) as usize] = data;
            }
        } else if EXTERNAL_RAM.contains(&address) {
            // self.game_rom.write(address, data);
        } else if ECHO_RAM.contains(&address) {
            self.data[address as usize] = data;
            self.data[(WORK_RAM_1.min().unwrap() + (address - ECHO_RAM.min().unwrap())) as usize] = data; // Copy to the working RAM
        } else if address == TIMER_DIVIDER_REGISTER_ADDRESS {
            self.timer.borrow_mut().reset();
        } else if address == LCD_CONTROL_ADDRESS {
            self.data[address as usize] = data;
            // Check if LCD is being turned on or off
            if (get_bit(data, BitIndex::I7) && !get_bit(self.data[address as usize], BitIndex::I7)) ||
               !get_bit(data, BitIndex::I7) {
                self.data[LCD_Y_ADDRESS as usize] = 0x00;
                // Set Hblank
                let byte = self.data[LCD_STATUS_ADDRESS as usize];
                self.data[LCD_STATUS_ADDRESS as usize] = byte & 0b11111100;
            }
        } else if address == LCD_Y_ADDRESS {
            // println!("Write to LCD_Y not allowed");
        } else if address == LCD_STATUS_ADDRESS {
            let byte = self.data[address as usize];
            self.data[address as usize] = (data & 0b11111000) | (byte & 0b00000111);
        } else if address == JOYPAD_ADDRESS {
            let byte = self.data[address as usize];
            self.data[address as usize] = (data & 0b11110000) | (byte & 0b00001111);
        } else if VIDEO_RAM.contains(&address) {
            return self.ppu.borrow_mut().write_vram(address, data);
        } else if SPRITE_ATTRIBUTE_TABLE.contains(&address) {
            return self.ppu.borrow_mut().write_oam(address, data);
        } else if address == DMA_ADDRESS {
            self.data[address as usize] = data;
            let source = (data as u16) * 0x100;
            let mut count: u16 = 0;
            let oam_addr = SPRITE_ATTRIBUTE_TABLE.min().unwrap();
            while count < 160 {
                self.ppu.borrow_mut().write_oam(oam_addr + count, self.data[(source + count) as usize]);
                count += 1;
            }
        } else {
            self.data[address as usize] = data;
        }
    }

    pub fn force_write(&mut self, address: u16, data: u8) {
        self.data[address as usize] = data;
    }

    pub fn write_16bit(&mut self, address: u16, data: u16) {
        let bytes = data.to_le_bytes();
        self.write(address, bytes[0]);
        self.write(address.wrapping_add(1), bytes[1]);
    }

    pub fn set_interrupt_flag(&mut self, interrupt: Interrupt, val: bool) {
        let byte = self.read(INTERRUPT_FLAG_ADDRESS);
        self.write(INTERRUPT_FLAG_ADDRESS, interrupt.set(byte, val));
    }
}
