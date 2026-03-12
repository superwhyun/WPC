#[cfg(windows)]
mod imp {
    use std::{ptr::null_mut, slice};

    use windows::Win32::{
        Foundation::{HLOCAL, LocalFree},
        Security::Cryptography::{
            CryptProtectData, CryptUnprotectData, CRYPTPROTECT_UI_FORBIDDEN, CRYPT_INTEGER_BLOB,
        },
    };

    use crate::{Error, Result};

    pub fn seal_bytes(input: &[u8]) -> Result<Vec<u8>> {
        unsafe {
            let mut in_blob = CRYPT_INTEGER_BLOB {
                cbData: input.len() as u32,
                pbData: input.as_ptr() as *mut u8,
            };
            let mut out_blob = CRYPT_INTEGER_BLOB::default();
            CryptProtectData(
                &mut in_blob,
                None,
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
            .map_err(|_| Error::SecretSeal)?;

            let bytes = slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out_blob.pbData.cast())));
            Ok(bytes)
        }
    }

    pub fn unseal_bytes(input: &[u8]) -> Result<Vec<u8>> {
        unsafe {
            let mut in_blob = CRYPT_INTEGER_BLOB {
                cbData: input.len() as u32,
                pbData: input.as_ptr() as *mut u8,
            };
            let mut out_blob = CRYPT_INTEGER_BLOB::default();
            CryptUnprotectData(
                &mut in_blob,
                Some(null_mut()),
                None,
                None,
                None,
                CRYPTPROTECT_UI_FORBIDDEN,
                &mut out_blob,
            )
            .map_err(|_| Error::SecretUnseal)?;

            let bytes = slice::from_raw_parts(out_blob.pbData, out_blob.cbData as usize).to_vec();
            let _ = LocalFree(Some(HLOCAL(out_blob.pbData.cast())));
            Ok(bytes)
        }
    }
}

#[cfg(not(windows))]
mod imp {
    use crate::Result;

    pub fn seal_bytes(input: &[u8]) -> Result<Vec<u8>> {
        Ok(input.to_vec())
    }

    pub fn unseal_bytes(input: &[u8]) -> Result<Vec<u8>> {
        Ok(input.to_vec())
    }
}

pub use imp::{seal_bytes, unseal_bytes};


