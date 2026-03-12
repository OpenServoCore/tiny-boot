use embedded_storage::nor_flash::{
    ErrorType, NorFlash, NorFlashError, NorFlashErrorKind, ReadNorFlash,
};

use crate::common::{APP_BASE, APP_PTR, APP_SIZE, FLASH_ERASE_SIZE, FLASH_WRITE_SIZE};
use crate::hal::flash;

#[derive(Debug)]
pub enum StorageError {
    NotAligned,
    OutOfBounds,
    Protected,
}

impl NorFlashError for StorageError {
    fn kind(&self) -> NorFlashErrorKind {
        match self {
            StorageError::NotAligned => NorFlashErrorKind::NotAligned,
            StorageError::OutOfBounds => NorFlashErrorKind::OutOfBounds,
            StorageError::Protected => NorFlashErrorKind::Other,
        }
    }
}

pub(crate) struct Storage {
    regs: ch32_metapac::flash::Flash,
}

struct StorageUnlocked<'a> {
    regs: &'a ch32_metapac::flash::Flash,
}

impl Drop for StorageUnlocked<'_> {
    fn drop(&mut self) {
        flash::lock(self.regs);
    }
}

impl Storage {
    pub fn new(regs: ch32_metapac::flash::Flash) -> Self {
        Storage { regs }
    }

    fn unlock(&self) -> StorageUnlocked<'_> {
        flash::unlock(&self.regs);
        StorageUnlocked { regs: &self.regs }
    }
}

fn check_error(regs: &ch32_metapac::flash::Flash) -> Result<(), StorageError> {
    if flash::check_wrprterr(regs) {
        Err(StorageError::Protected)
    } else {
        Ok(())
    }
}

impl ErrorType for Storage {
    type Error = StorageError;
}

impl NorFlash for Storage {
    const WRITE_SIZE: usize = FLASH_WRITE_SIZE;
    const ERASE_SIZE: usize = FLASH_ERASE_SIZE;

    fn erase(&mut self, from: u32, to: u32) -> Result<(), Self::Error> {
        if from as usize % FLASH_ERASE_SIZE != 0 || to as usize % FLASH_ERASE_SIZE != 0 {
            return Err(StorageError::NotAligned);
        }
        if to as usize > APP_SIZE {
            return Err(StorageError::OutOfBounds);
        }
        let _guard = self.unlock();
        let mut addr = APP_BASE + from;
        let end = APP_BASE + to;
        while addr < end {
            flash::erase_page(&self.regs, addr);
            check_error(&self.regs)?;
            addr += FLASH_ERASE_SIZE as u32;
        }
        Ok(())
    }

    fn write(&mut self, offset: u32, bytes: &[u8]) -> Result<(), Self::Error> {
        if offset as usize % FLASH_WRITE_SIZE != 0 || bytes.len() % FLASH_WRITE_SIZE != 0 {
            return Err(StorageError::NotAligned);
        }
        if offset as usize + bytes.len() > APP_SIZE {
            return Err(StorageError::OutOfBounds);
        }
        let _guard = self.unlock();
        let mut addr = APP_BASE + offset;
        for chunk in bytes.chunks_exact(FLASH_WRITE_SIZE) {
            flash::write_page(&self.regs, addr, chunk);
            check_error(&self.regs)?;
            addr += FLASH_WRITE_SIZE as u32;
        }
        Ok(())
    }
}

impl ReadNorFlash for Storage {
    const READ_SIZE: usize = 1;

    fn read(&mut self, offset: u32, bytes: &mut [u8]) -> Result<(), Self::Error> {
        if offset as usize + bytes.len() > APP_SIZE {
            return Err(StorageError::OutOfBounds);
        }
        let src = unsafe { core::slice::from_raw_parts(APP_PTR as *const u8, APP_SIZE) };
        let offset = offset as usize;
        bytes.copy_from_slice(&src[offset..offset + bytes.len()]);
        Ok(())
    }

    fn capacity(&self) -> usize {
        APP_SIZE
    }
}
