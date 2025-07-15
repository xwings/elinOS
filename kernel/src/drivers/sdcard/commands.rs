// SD card command definitions for elinOS
// Implements MMC/SD card command protocol

/// SD card command structure
#[derive(Debug, Clone, Copy)]
pub struct SdCommand {
    pub cmd: u8,
    pub arg: u32,
    pub crc: u8,
}

impl SdCommand {
    /// Create a new SD command
    pub fn new(cmd: u8, arg: u32) -> Self {
        let crc = Self::calculate_crc(cmd, arg);
        SdCommand { cmd, arg, crc }
    }
    
    /// Convert command to bytes for SPI transmission
    pub fn to_bytes(&self) -> [u8; 6] {
        [
            0x40 | self.cmd,                    // Command with start bit
            (self.arg >> 24) as u8,             // Argument bits 31-24
            (self.arg >> 16) as u8,             // Argument bits 23-16
            (self.arg >> 8) as u8,              // Argument bits 15-8
            self.arg as u8,                     // Argument bits 7-0
            (self.crc << 1) | 1,                // CRC with end bit
        ]
    }
    
    /// Calculate CRC7 for SD command
    fn calculate_crc(cmd: u8, arg: u32) -> u8 {
        let mut crc = 0u8;
        let data = [
            0x40 | cmd,
            (arg >> 24) as u8,
            (arg >> 16) as u8,
            (arg >> 8) as u8,
            arg as u8,
        ];
        
        for &byte in &data {
            crc = Self::crc7_byte(crc, byte);
        }
        
        crc
    }
    
    /// Calculate CRC7 for a single byte
    fn crc7_byte(crc: u8, data: u8) -> u8 {
        let mut crc = crc;
        let mut data = data;
        
        for _ in 0..8 {
            crc <<= 1;
            if (data & 0x80) ^ (crc & 0x80) != 0 {
                crc ^= 0x09;
            }
            data <<= 1;
        }
        
        crc & 0x7F
    }
}

// SD card command definitions
pub const CMD0_GO_IDLE_STATE: u8 = 0;
pub const CMD1_SEND_OP_COND: u8 = 1;
pub const CMD8_SEND_IF_COND: u8 = 8;
pub const CMD9_SEND_CSD: u8 = 9;
pub const CMD10_SEND_CID: u8 = 10;
pub const CMD12_STOP_TRANSMISSION: u8 = 12;
pub const CMD16_SET_BLOCKLEN: u8 = 16;
pub const CMD17_READ_SINGLE_BLOCK: u8 = 17;
pub const CMD18_READ_MULTIPLE_BLOCK: u8 = 18;
pub const CMD24_WRITE_BLOCK: u8 = 24;
pub const CMD25_WRITE_MULTIPLE_BLOCK: u8 = 25;
pub const CMD32_ERASE_WR_BLK_START: u8 = 32;
pub const CMD33_ERASE_WR_BLK_END: u8 = 33;
pub const CMD38_ERASE: u8 = 38;
pub const CMD55_APP_CMD: u8 = 55;
pub const CMD58_READ_OCR: u8 = 58;
pub const CMD59_CRC_ON_OFF: u8 = 59;

// Application-specific commands (preceded by CMD55)
pub const ACMD13_SD_STATUS: u8 = 13;
pub const ACMD22_SEND_NUM_WR_BLOCKS: u8 = 22;
pub const ACMD23_SET_WR_BLK_ERASE_COUNT: u8 = 23;
pub const ACMD41_SD_SEND_OP_COND: u8 = 41;
pub const ACMD42_SET_CLR_CARD_DETECT: u8 = 42;
pub const ACMD51_SEND_SCR: u8 = 51;

// SD card response types
pub const R1_IDLE_STATE: u8 = 0x01;
pub const R1_ERASE_RESET: u8 = 0x02;
pub const R1_ILLEGAL_COMMAND: u8 = 0x04;
pub const R1_COM_CRC_ERROR: u8 = 0x08;
pub const R1_ERASE_SEQUENCE_ERROR: u8 = 0x10;
pub const R1_ADDRESS_ERROR: u8 = 0x20;
pub const R1_PARAMETER_ERROR: u8 = 0x40;
pub const R1_MSB: u8 = 0x80;

// Data response tokens
pub const DATA_START_TOKEN: u8 = 0xFE;
pub const DATA_START_TOKEN_MULTI: u8 = 0xFC;
pub const DATA_STOP_TOKEN_MULTI: u8 = 0xFD;
pub const DATA_ERROR_TOKEN: u8 = 0x01;

// Data response masks
pub const DATA_RESPONSE_MASK: u8 = 0x1F;
pub const DATA_RESPONSE_ACCEPTED: u8 = 0x05;
pub const DATA_RESPONSE_CRC_ERROR: u8 = 0x0B;
pub const DATA_RESPONSE_WRITE_ERROR: u8 = 0x0D;

/// SD card response structure
#[derive(Debug, Clone, Copy)]
pub struct SdResponse {
    pub r1: u8,
    pub data: [u8; 4],
    pub length: usize,
}

impl SdResponse {
    pub fn new(r1: u8) -> Self {
        SdResponse {
            r1,
            data: [0; 4],
            length: 1,
        }
    }
    
    pub fn with_data(r1: u8, data: [u8; 4]) -> Self {
        SdResponse {
            r1,
            data,
            length: 5,
        }
    }
    
    pub fn is_valid(&self) -> bool {
        (self.r1 & R1_MSB) == 0
    }
    
    pub fn is_idle(&self) -> bool {
        (self.r1 & R1_IDLE_STATE) != 0
    }
    
    pub fn has_error(&self) -> bool {
        (self.r1 & 0x7E) != 0
    }
}