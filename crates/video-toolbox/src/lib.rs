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

#[derive(Copy, Clone, Debug, PartialEq, Hash, Eq)]
pub(crate) enum NalType {
    // Reference: https://www.itu.int/rec/T-REC-H.265-202108-I/en

    // BLA = Broken Link Access
    // CRA = Clean Random Access
    // IDR = Instantaneous Decoding Refresh
    // IRAP = Intra Random Access Point
    // NAL = Network Abstraction Layer
    // NALU = Network Abstraction Layer Unit
    // RADL = Random Access Decodable Leading
    // RASL = Random Access Skipped Leading
    // SLNR = Sub-Layer Non-Reference
    // STSA = Step-wise Temporal Sub-layer Access
    // TSA = Temporal Sub-layer Access
    // VCL = Video Coding Layer

    // VCL NAL types
    CodedSliceTrailN, // Coded slice segment of a non-TSA, non-STSA trailing picture, Non-reference
    CodedSliceTrailR, // Coded slice segment of a non-TSA, non-STSA trailing picture, Reference

    CodedSliceTsaN, // Coded slice segment of a TSA picture, Non-reference
    CodedSliceTsaR, // Coded slice segment of a TSA picture, Reference

    CodedSliceStsaN, // Coded slice segment of an STSA picture, Non-reference
    CodedSliceStsaR, // Coded slice segment of an STSA picture, Reference

    CodedSliceRadlN, // Coded slice segment of a RADL picture, Non-reference
    CodedSliceRadlR, // Coded slice segment of a RADL picture, Reference

    CodedSliceRaslN, // Coded slice segment of a RASL picture, Non-reference
    CodedSliceRaslR, // Coded slice segment of a RASL picture, Reference

    ReservedVclN10, // Reserved non-IRAP SLNR VCL NAL unit types
    ReservedVclN11, // Reserved non-IRAP sub-layer reference VCL NAL unit types
    ReservedVclN12, // Reserved non-IRAP SLNR VCL NAL unit types
    ReservedVclN13, // Reserved non-IRAP sub-layer reference VCL NAL unit types
    ReservedVclN14, // Reserved non-IRAP SLNR VCL NAL unit types
    ReservedVclN15, // Reserved non-IRAP sub-layer reference VCL NAL unit types

    CodedSliceBlaWLp,   // Coded slice segment of a BLA picture
    CodedSliceBlaWRadl, // Coded slice segment of a BLA picture
    CodedSliceBlaNLp,   // Coded slice segment of a BLA picture

    CodedSliceIdrWRadl, // Coded slice segment of an IDR picture
    CodedSliceIdrNLp,   // Coded slice segment of an IDR picture

    CodedSliceCra, // Coded slice segment of a CRA picture

    ReservedIrapVcl22, // Reserved IRAP VCL NAL unit types
    ReservedIrapVcl23, // Reserved IRAP VCL NAL unit types

    ReservedVcl24, // Reserved non-IRAP VCL NAL unit types
    ReservedVcl25, // Reserved non-IRAP VCL NAL unit types
    ReservedVcl26, // Reserved non-IRAP VCL NAL unit types
    ReservedVcl27, // Reserved non-IRAP VCL NAL unit types
    ReservedVcl28, // Reserved non-IRAP VCL NAL unit types
    ReservedVcl29, // Reserved non-IRAP VCL NAL unit types
    ReservedVcl30, // Reserved non-IRAP VCL NAL unit types
    ReservedVcl31, // Reserved non-IRAP VCL NAL unit types

    // Non-VCL NAL Units
    Vps, // Video parameter set
    Sps, // Sequence parameter set
    Pps, // Picture parameter set

    Aud, // Access unit delimiter

    Eos, // End of sequence
    Eob, // End of bitstream

    Fd, // Filler data

    PrefixSei, // Supplemental enhancement information
    SuffixSei, // Supplemental enhancement information

    ReservedNvcl41, // Reserved
    ReservedNvcl42, // Reserved
    ReservedNvcl43, // Reserved
    ReservedNvcl44, // Reserved
    ReservedNvcl45, // Reserved
    ReservedNvcl46, // Reserved
    ReservedNvcl47, // Reserved

    Unspecified48, // Unspecified
    Unspecified49, // Unspecified
    Unspecified50, // Unspecified
    Unspecified51, // Unspecified
    Unspecified52, // Unspecified
    Unspecified53, // Unspecified
    Unspecified54, // Unspecified
    Unspecified55, // Unspecified
    Unspecified56, // Unspecified
    Unspecified57, // Unspecified
    Unspecified58, // Unspecified
    Unspecified59, // Unspecified
    Unspecified60, // Unspecified
    Unspecified61, // Unspecified
    Unspecified62, // Unspecified
    Unspecified63, // Unspecified

    Unknown(u8),
}

impl From<u8> for NalType {
    fn from(nal_byte: u8) -> Self {
        match nal_byte {
            0 => NalType::CodedSliceTrailN,
            1 => NalType::CodedSliceTrailR,
            2 => NalType::CodedSliceTsaN,
            3 => NalType::CodedSliceTsaR,
            4 => NalType::CodedSliceStsaN,
            5 => NalType::CodedSliceStsaR,
            6 => NalType::CodedSliceRadlN,
            7 => NalType::CodedSliceRadlR,
            8 => NalType::CodedSliceRaslN,
            9 => NalType::CodedSliceRaslR,
            10 => NalType::ReservedVclN10,
            11 => NalType::ReservedVclN11,
            12 => NalType::ReservedVclN12,
            13 => NalType::ReservedVclN13,
            14 => NalType::ReservedVclN14,
            15 => NalType::ReservedVclN15,
            16 => NalType::CodedSliceBlaWLp,
            17 => NalType::CodedSliceBlaWRadl,
            18 => NalType::CodedSliceBlaNLp,
            19 => NalType::CodedSliceIdrWRadl,
            20 => NalType::CodedSliceIdrNLp,
            21 => NalType::CodedSliceCra,
            22 => NalType::ReservedIrapVcl22,
            23 => NalType::ReservedIrapVcl23,
            24 => NalType::ReservedVcl24,
            25 => NalType::ReservedVcl25,
            26 => NalType::ReservedVcl26,
            27 => NalType::ReservedVcl27,
            28 => NalType::ReservedVcl28,
            29 => NalType::ReservedVcl29,
            30 => NalType::ReservedVcl30,
            31 => NalType::ReservedVcl31,
            32 => NalType::Vps,
            33 => NalType::Sps,
            34 => NalType::Pps,
            35 => NalType::Aud,
            36 => NalType::Eos,
            37 => NalType::Eob,
            38 => NalType::Fd,
            39 => NalType::PrefixSei,
            40 => NalType::SuffixSei,
            41 => NalType::ReservedNvcl41,
            42 => NalType::ReservedNvcl42,
            43 => NalType::ReservedNvcl43,
            44 => NalType::ReservedNvcl44,
            45 => NalType::ReservedNvcl45,
            46 => NalType::ReservedNvcl46,
            47 => NalType::ReservedNvcl47,
            48 => NalType::Unspecified48,
            49 => NalType::Unspecified49,
            50 => NalType::Unspecified50,
            51 => NalType::Unspecified51,
            52 => NalType::Unspecified52,
            53 => NalType::Unspecified53,
            54 => NalType::Unspecified54,
            55 => NalType::Unspecified55,
            56 => NalType::Unspecified56,
            57 => NalType::Unspecified57,
            58 => NalType::Unspecified58,
            59 => NalType::Unspecified59,
            60 => NalType::Unspecified60,
            61 => NalType::Unspecified61,
            62 => NalType::Unspecified62,
            63 => NalType::Unspecified63,
            byte => NalType::Unknown(byte),
        }
    }
}

struct Nal<'a> {
    nal_type: NalType,
    data: &'a [u8],
}
