use crate::sys::pal::abi;

pub fn fill_bytes(bytes: &mut [u8]) {
    unsafe { abi::__dysnomia_pal_v1_fill_bytes(bytes.as_mut_ptr(), bytes.len() as u64) }
}
