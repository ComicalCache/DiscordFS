use aes_gcm_siv::Nonce;

pub struct NonceCounter(u64);

impl NonceCounter {
    pub fn new() -> Self {
        NonceCounter(0)
    }

    pub fn get_nonce(&mut self) -> Nonce {
        let mut data = [0; 12];
        data[..4].copy_from_slice(&0u32.to_le_bytes());
        data[4..].copy_from_slice(&self.0.to_le_bytes());

        self.0 += 1;

        *Nonce::from_slice(&data)
    }
}
