//! ELF Error Types and Result Definitions
//!
//! This module provides error handling for ELF parsing, loading, and execution.

use core::fmt;
use crate::console_println;

/// ELF Loading and Execution Errors
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ElfError {
    InvalidMagic,
    UnsupportedClass,
    UnsupportedEndian,
    UnsupportedMachine,
    UnsupportedType,
    InvalidHeader,
    LoadError,
    ExecutionError,
    MemoryAllocationFailed,
    InvalidEntryPoint,
}

impl fmt::Display for ElfError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        console_println!("!! ElfError Formatted: {:?}", self);
        match self {
            ElfError::InvalidMagic => write!(f, "Invalid ELF magic number"),
            ElfError::UnsupportedClass => write!(f, "Unsupported ELF class"),
            ElfError::UnsupportedEndian => write!(f, "Unsupported endianness"),
            ElfError::UnsupportedMachine => write!(f, "Unsupported machine type"),
            ElfError::UnsupportedType => write!(f, "Unsupported ELF type"),
            ElfError::InvalidHeader => write!(f, "Invalid ELF header"),
            ElfError::LoadError => write!(f, "ELF loading error"),
            ElfError::ExecutionError => write!(f, "ELF execution error"),
            ElfError::MemoryAllocationFailed => write!(f, "Memory allocation failed"),
            ElfError::InvalidEntryPoint => write!(f, "Invalid entry point"),
        }
    }
}

/// Result type for ELF operations
pub type ElfResult<T> = Result<T, ElfError>; 