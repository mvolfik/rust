pub fn fill_bytes(bytes: &mut [u8]) {
    for byte in bytes.iter_mut() {
        *byte = unsafe { libc::rand() as u8 };
    }
}
