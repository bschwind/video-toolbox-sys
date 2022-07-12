use thiserror::Error;

mod decoder;
mod encoder;

pub use decoder::*;
pub use encoder::*;

#[derive(Debug, Error)]
pub enum HevcError {
    #[error("Invalid NAL Type: {0}")]
    InvalidNalType(u8),
}

pub(crate) struct NalIterator<'a> {
    hevc_bytes: &'a [u8],
}

impl<'a> NalIterator<'a> {
    fn new(hevc_bytes: &'a [u8]) -> Self {
        let mut cursor = 0;

        while cursor < hevc_bytes.len() && hevc_bytes[cursor] != 1 {
            cursor += 1;
        }

        if cursor + 1 >= hevc_bytes.len() {
            return Self { hevc_bytes: &[] };
        }

        cursor += 1;
        Self { hevc_bytes: &hevc_bytes[cursor..] }
    }

    fn next_header(data: &[u8]) -> Option<(usize, usize)> {
        if data.len() < 3 {
            return None;
        }

        for i in 2..(data.len() - 1) {
            if data[i] == 1 {
                let last_two_are_zero = data[i - 1] == 0 && data[i - 2] == 0;

                if last_two_are_zero {
                    if data[i - 3] == 0 {
                        return Some((i - 3, i));
                    } else {
                        return Some((i - 2, i));
                    }
                }
            }
        }

        None
    }
}

impl<'a> Iterator for NalIterator<'a> {
    type Item = Nal<'a>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.hevc_bytes.is_empty() {
            return None;
        }

        let nal_type_byte = (self.hevc_bytes[0] >> 1) & 0b0011_1111;
        let nal_type = NalType::from(nal_type_byte);

        if let Some((next_header_start, next_header_end)) = Self::next_header(self.hevc_bytes) {
            let nal = Nal { nal_type, data: &self.hevc_bytes[..next_header_start] };

            self.hevc_bytes = &self.hevc_bytes[(next_header_end + 1)..];

            Some(nal)
        } else {
            let nal = Nal { nal_type, data: self.hevc_bytes };

            self.hevc_bytes = &[];

            Some(nal)
        }
    }
}

#[derive(Debug, PartialEq)]
pub(crate) enum NalType {
    CodedSliceTrailR,   // P-frame
    CodedSliceIdrNLp,   // I-frame
    CodedSliceCra,      // I-frame?
    CodedSliceIdrWRadl, // I-frame
    Vps,
    Sps,
    Pps,
    Unknown(u8),
}

impl From<u8> for NalType {
    fn from(nal_byte: u8) -> Self {
        match nal_byte {
            1 => NalType::CodedSliceTrailR,
            20 => NalType::CodedSliceIdrNLp,
            21 => NalType::CodedSliceCra,
            19 => NalType::CodedSliceIdrWRadl,
            32 => NalType::Vps,
            33 => NalType::Sps,
            34 => NalType::Pps,
            byte => NalType::Unknown(byte),
        }
    }
}

struct Nal<'a> {
    nal_type: NalType,
    data: &'a [u8],
}
