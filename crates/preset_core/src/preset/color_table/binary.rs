//! Das Format im Detail: Adobes Base85-Alphabet und ein Leser für
//! Zahlen im Little-Endian-Format.

use super::ALPHABET;

/// Base85 nach Adobe: je 5 Zeichen → 4 Bytes (Little Endian). Ein kürzerer
/// Rest ergibt entsprechend weniger Bytes.
pub(super) fn base85(text: &str) -> Option<Vec<u8>> {
    let mut lookup = [u8::MAX; 256];
    for (index, &character) in ALPHABET.iter().enumerate() {
        lookup[usize::from(character)] = index as u8;
    }
    let digits: Vec<u8> = text
        .bytes()
        .filter(|byte| !byte.is_ascii_whitespace())
        .map(|byte| lookup[usize::from(byte)])
        .collect();
    if digits.contains(&u8::MAX) {
        return None;
    }
    let mut bytes = Vec::with_capacity(digits.len() / 5 * 4 + 4);
    for chunk in digits.chunks(5) {
        let value = chunk
            .iter()
            .rev()
            .fold(0u64, |value, &digit| value * 85 + u64::from(digit));
        let word = (value as u32).to_le_bytes();
        let take = if chunk.len() == 5 {
            4
        } else {
            chunk.len().saturating_sub(1)
        };
        bytes.extend_from_slice(&word[..take]);
    }
    Some(bytes)
}

/// Liest Zahlen aus der entpackten Tabelle.
pub(super) struct Reader<'a> {
    pub(super) data: &'a [u8],
    pub(super) at: usize,
}

impl Reader<'_> {
    /// Wie viele Bytes noch ungelesen sind.
    pub(super) fn remaining(&self) -> usize {
        self.data.len().saturating_sub(self.at)
    }

    pub(super) fn take<const N: usize>(&mut self) -> Option<[u8; N]> {
        let bytes = self.data.get(self.at..self.at + N)?;
        self.at += N;
        bytes.try_into().ok()
    }
    pub(super) fn u16(&mut self) -> Option<u16> {
        self.take().map(u16::from_le_bytes)
    }
    pub(super) fn u32(&mut self) -> Option<u32> {
        self.take().map(u32::from_le_bytes)
    }
    pub(super) fn f32(&mut self) -> Option<f32> {
        self.take().map(f32::from_le_bytes)
    }
    pub(super) fn f64(&mut self) -> Option<f64> {
        self.take().map(f64::from_le_bytes)
    }
}
