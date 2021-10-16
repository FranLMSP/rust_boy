use crate::utils::{join_bytes};
use crate::rom::ROM;

pub enum MemoryMap {
    BankZero,
    BankSwitchable,
    VideoRAM,
    ExternalRAM,
    WorkRAM1,
    WorkRAM2,
    EchoRAM,
    SpriteAttributeTable,
    NotUsable,
    IORegisters,
    HighRAM,
    InterruptEnableRegister,
}

impl MemoryMap {
    pub fn get_map(address: u16) -> Self {
        match address {
            0x0000..=0x3FFF => Self::BankZero,
            0x4000..=0x7FFF => Self::BankSwitchable,
            0x8000..=0x9FFF => Self::VideoRAM,
            0xA000..=0xBFFF => Self::ExternalRAM,
            0xC000..=0xCFFF => Self::WorkRAM1,
            0xD000..=0xDFFF => Self::WorkRAM2,
            0xE000..=0xFDFF => Self::EchoRAM, // Mirror of C000~DDFF
            0xFE00..=0xFE9F => Self::SpriteAttributeTable,
            0xFEA0..=0xFEFF => Self::NotUsable,
            0xFF00..=0xFF7F => Self::IORegisters,
            0xFF80..=0xFFFE => Self::HighRAM,
            0xFFFF => Self::InterruptEnableRegister,
            _  => Self::BankZero,
        }
    }
}

pub struct Bus {
    game_rom: ROM,
    data: [u8; 0x10000],
}

impl Bus {
    pub fn new() -> Self {
        let game_rom = match ROM::load_file("roms/cpu_instrs_individual/01-special.gb".to_string()) {
            Ok(rom) => rom,
            _ => ROM::from_bytes(&[0; 0xFFFF])
        };
        Self {
            data: [0; 0x10000],
            game_rom,
        }
    }

    pub fn read(&self, address: u16) -> u8 {
        match MemoryMap::get_map(address) {
            MemoryMap::BankZero => self.game_rom.read(address),
            MemoryMap::BankSwitchable => self.game_rom.read(address),
            // MemoryMap::InterruptEnableRegister => self.data[address as usize],
            _ => self.data[address as usize],
        }
    }

    pub fn read_16bit(&self, address: u16) -> u16 {
        join_bytes(self.read(address + 1), self.read(address))
    }

    pub fn write(&mut self, address: u16, data: u8) {
        self.data[address as usize] = data;
    }

    pub fn write_16bit(&mut self, address: u16, data: u16) {
        let bytes = data.to_be_bytes();
        self.write(address, bytes[1]);
        self.write(address + 1, bytes[0]);
    }
}
