use core::ffi::c_void;
use core_foundation::{
    array::CFArrayGetValueAtIndex,
    base::{CFIndexConvertible, OSStatus},
    boolean::CFBoolean,
    dictionary::{
        kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate,
        CFDictionarySetValue,
    },
    number::kCFBooleanTrue,
    string::CFStringRef,
};
use std::convert::TryInto;
use video_toolbox_sys::{
    kCMSampleAttachmentKey_DisplayImmediately,
    kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder,
    CMBlockBufferCreateWithMemoryBlock, CMBlockBufferRef, CMSampleBufferCreate,
    CMSampleBufferGetSampleAttachmentsArray, CMSampleBufferRef, CMTime,
    CMVideoFormatDescriptionCreateFromHEVCParameterSets, CMVideoFormatDescriptionRef,
    CVImageBufferRef, VTDecodeInfoFlags, VTDecompressionOutputCallbackRecord,
    VTDecompressionSessionCreate, VTDecompressionSessionDecodeFrame, VTDecompressionSessionRef,
};

extern "C" fn decode_callback(
    _output_callback_ref_con: *mut c_void,
    _source_frame_ref_con: *mut c_void,
    status: OSStatus,
    _info_flags: VTDecodeInfoFlags,
    _image_buffer: CVImageBufferRef,
    _presentation_timestamp: CMTime,
    _presentation_duration: CMTime,
) {
    println!("decode_callback");
    println!("Status: {}", status);
}

struct NalIterator<'a> {
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
}

impl<'a> Iterator for NalIterator<'a> {
    type Item = Nal<'a>;

    fn next(&mut self) -> Option<<Self as Iterator>::Item> {
        if self.hevc_bytes.is_empty() {
            return None;
        }

        let nal_type = (self.hevc_bytes[0] >> 1) & 0b0011_1111;

        if let Some((next_header_start, next_header_end)) = next_header(&self.hevc_bytes) {
            let nal = Nal { nal_type, data: &self.hevc_bytes[..next_header_start] };

            self.hevc_bytes = &self.hevc_bytes[(next_header_end + 1)..];

            Some(nal)
        } else {
            let nal = Nal { nal_type, data: &self.hevc_bytes };

            self.hevc_bytes = &[];

            Some(nal)
        }
    }
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

// #[repr(u8)]
// enum NalType {
//     Vps = 32,
//     Sps = 33,
//     Pps = 34,
// }

struct Nal<'a> {
    // nal_type: NalType,
    nal_type: u8,
    data: &'a [u8],
}

fn main() {
    let hevc_bytes = include_bytes!("../out.hevc");

    let mut vps_slice: Option<&[u8]> = None;
    let mut sps_slice: Option<&[u8]> = None;
    let mut pps_slice: Option<&[u8]> = None;
    let mut idr_slice: Option<&[u8]> = None;

    let nal_iter = NalIterator::new(hevc_bytes);

    for nal in nal_iter {
        println!("NAL: {:?}, size: {}", nal.nal_type, nal.data.len());

        if nal.nal_type == 32 {
            vps_slice = Some(nal.data);
        }

        if nal.nal_type == 33 {
            sps_slice = Some(nal.data);
        }

        if nal.nal_type == 34 {
            pps_slice = Some(nal.data);
        }

        if nal.nal_type == 20 {
            idr_slice = Some(nal.data);
        }
    }

    let _frame_width = 1280usize;
    let _frame_height = 720usize;

    let keys: Vec<CFStringRef> =
        unsafe { vec![kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder] };
    let values: Vec<CFBoolean> = vec![CFBoolean::true_value()];

    let decoder_specification = unsafe {
        CFDictionaryCreate(
            std::ptr::null(),
            std::mem::transmute(keys.as_ptr()),
            std::mem::transmute(values.as_ptr()),
            keys.len().to_CFIndex().try_into().unwrap(),
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };

    let format_description = unsafe {
        let mut format_ref = std::mem::MaybeUninit::<CMVideoFormatDescriptionRef>::uninit();

        let vps = vps_slice.unwrap();
        let sps = sps_slice.unwrap();
        let pps = pps_slice.unwrap();

        let parameter_set_sizes = vec![vps.len(), sps.len(), pps.len()];
        let parameter_sets = vec![vps.as_ptr(), sps.as_ptr(), pps.as_ptr()];

        CMVideoFormatDescriptionCreateFromHEVCParameterSets(
            std::ptr::null(),     // Allocator
            parameter_sets.len(), // parameter set count
            parameter_sets.as_ptr(),
            parameter_set_sizes.as_ptr(),
            4,                                                      // NAL unit header length
            std::ptr::null(),                                       // extensions
            format_ref.as_mut_ptr() as CMVideoFormatDescriptionRef, // Format ref out
        );

        let format = format_ref.assume_init();

        format
    };

    // https://github.com/peter-iakovlev/TelegramUI/blob/e8b193443d1b84f00390138a82c44ebfcceb496a/TelegramUI/FFMpegMediaFrameSourceContextHelpers.swift#L67-L92
    // https://stackoverflow.com/questions/29525000/how-to-use-videotoolbox-to-decompress-h-264-video-stream/29525001#29525001

    // Create the decoder
    let mut decompression_ref = std::mem::MaybeUninit::<VTDecompressionSessionRef>::uninit();

    let callback_record = VTDecompressionOutputCallbackRecord {
        decompression_output_callback: Some(decode_callback),
        decompression_output_ref_con: std::ptr::null_mut(),
    };

    let create_status = unsafe {
        VTDecompressionSessionCreate(
            std::ptr::null(),                                            // Allocator
            format_description,                                          // Format Description
            decoder_specification,                                       // Decoder specification,
            std::ptr::null(), // Dest image buffer attributes
            &callback_record, // Output callback, pass NULL if you're using VTDecompressionSessionDecodeFrameWithOutputHandler
            decompression_ref.as_mut_ptr() as VTDecompressionSessionRef, // Decompression session out
        )
    };

    if create_status != 0 {
        println!("Failed to create VT Compression Session: {}", create_status);
        return;
    }

    let decompression_session = unsafe { decompression_ref.assume_init() };

    let frame_data = idr_slice.expect("Should have frame data");

    let mut length_prefixed_data = vec![];
    length_prefixed_data.extend_from_slice(&(frame_data.len() as u32).to_be_bytes());
    length_prefixed_data.extend_from_slice(frame_data);
    let frame_data = length_prefixed_data;

    let block_buffer = unsafe {
        let mut block_buffer_out = std::mem::MaybeUninit::<CMBlockBufferRef>::uninit();

        let status = CMBlockBufferCreateWithMemoryBlock(
            std::ptr::null(),                                  // Allocator
            frame_data.as_ptr() as *const c_void,              // Memory block
            frame_data.len(),                                  // Block length
            std::ptr::null(),                                  // Block allocator
            std::ptr::null(),                                  // Custom block source
            0,                                                 // Offset to data
            frame_data.len(),                                  // Data length
            0,                                                 // Flags
            block_buffer_out.as_mut_ptr() as CMBlockBufferRef, // Block buffer out
        );

        if status != 0 {
            println!("Error creating CMBlockBuffer");
        }

        block_buffer_out.assume_init()
    };

    let sample_buffer = unsafe {
        let sample_size = frame_data.len();
        let mut sample_buffer_out = std::mem::MaybeUninit::<CMSampleBufferRef>::uninit();

        let status = CMSampleBufferCreate(
            std::ptr::null(),                                    // Allocator
            block_buffer,                                        // Data
            true,                                                // Data Ready
            None,                                                // Make data ready callback
            std::ptr::null_mut(),                                // Make data ready callback ref con
            format_description,                                  // Format description
            1,                                                   // Num samples
            0,                                                   // Num sample timing entries
            std::ptr::null(),                                    // Sample timing array
            1,                                                   // Num sample timing entries
            &sample_size,                                        // Sample size
            sample_buffer_out.as_mut_ptr() as CMSampleBufferRef, // Sample buffer out
        );

        if status != 0 {
            println!("Error creating CMSampleBuffer");
        }

        sample_buffer_out.assume_init()
    };

    let attachments = unsafe { CMSampleBufferGetSampleAttachmentsArray(sample_buffer, true) };
    let dict = unsafe { CFArrayGetValueAtIndex(attachments, 0) };
    unsafe {
        CFDictionarySetValue(
            dict as *mut _,
            kCMSampleAttachmentKey_DisplayImmediately as *const c_void,
            kCFBooleanTrue as *const c_void,
        );
    }

    unsafe {
        VTDecompressionSessionDecodeFrame(
            decompression_session,
            sample_buffer,
            0,                    // Decode flags
            std::ptr::null(),     // User data
            std::ptr::null_mut(), // Info flags out
        );
    }
}
