use crate::{NalIterator, NalType};
use core::ffi::c_void;
use core_foundation::{
    array::CFArrayGetValueAtIndex,
    base::{CFIndexConvertible, OSStatus},
    boolean::CFBoolean,
    dictionary::{
        kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate,
        CFDictionaryCreateMutable, CFDictionarySetValue,
    },
    number::{kCFBooleanTrue, kCFNumberSInt32Type, CFNumberCreate},
    string::CFStringRef,
};
use thiserror::Error;
use video_toolbox_sys::{
    kCMSampleAttachmentKey_DisplayImmediately, kCVPixelBufferIOSurfacePropertiesKey,
    kCVPixelBufferLock_ReadOnly, kCVPixelBufferPixelFormatTypeKey, kCVPixelFormatType_32BGRA,
    kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder,
    CMBlockBufferCreateWithMemoryBlock, CMBlockBufferRef, CMSampleBufferCreate,
    CMSampleBufferGetSampleAttachmentsArray, CMSampleBufferRef, CMTime,
    CMVideoFormatDescriptionCreateFromHEVCParameterSets, CMVideoFormatDescriptionRef,
    CVImageBufferGetDisplaySize, CVImageBufferGetEncodedSize, CVImageBufferRef,
    CVPixelBufferGetBaseAddressOfPlane, CVPixelBufferGetBytesPerRowOfPlane,
    CVPixelBufferGetDataSize, CVPixelBufferGetHeight, CVPixelBufferGetHeightOfPlane,
    CVPixelBufferGetPixelFormatType, CVPixelBufferGetPlaneCount, CVPixelBufferGetWidth,
    CVPixelBufferIsPlanar, CVPixelBufferLockBaseAddress, CVPixelBufferUnlockBaseAddress,
    VTDecodeInfoFlags, VTDecompressionOutputCallbackRecord, VTDecompressionSessionCreate,
    VTDecompressionSessionDecodeFrame, VTDecompressionSessionRef,
    VTDecompressionSessionWaitForAsynchronousFrames,
};

#[derive(Debug, Error)]
pub enum DecodeError {
    #[error("Initialization Error: {0}")]
    InitializationError(i32),

    #[error("Missing Vps NAL Unit")]
    MissingVpsNalUnit,

    #[error("Missing Sps NAL Unit")]
    MissingSpsNalUnit,

    #[error("Missing Pps NAL Unit")]
    MissingPpsNalUnit,

    #[error("An intermediate frame was received before an I frame")]
    MissingIFrame,

    #[error("No intermediate frames were in the data payload")]
    MissingPFrame,

    #[error("Block Buffer Creation Error: {0}")]
    BlockBufferCreationError(i32),

    #[error("Sample Buffer Creation Error: {0}")]
    SampleBufferCreationError(i32),
}

pub struct Decoder {
    width: u32,
    height: u32,
    decoder_internal: DecoderInternal,
}

unsafe impl Send for Decoder {}

impl Drop for Decoder {
    fn drop(&mut self) {
        // TODO - call VTDecompressionSessionInvalidate
    }
}

impl Decoder {
    pub fn new(width: u32, height: u32) -> Result<Self, DecodeError> {
        Ok(Self { width, height, decoder_internal: DecoderInternal::new() })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    pub fn decode_blocking(&mut self, src: &[u8], dst: &mut [u8]) -> Result<usize, DecodeError> {
        self.decoder_internal.decode(src, dst)?;
        Ok((self.width * self.height * 4) as usize)
    }
}

struct DecoderInternal {
    decode_session: Option<VTDecompressionSessionRef>,
    format_description: Option<CMVideoFormatDescriptionRef>,
}

impl DecoderInternal {
    fn new() -> Self {
        Self { decode_session: None, format_description: None }
    }

    fn recreate_decoder(
        &mut self,
        vps_slice: &[u8],
        sps_slice: &[u8],
        pps_slice: &[u8],
    ) -> Result<(), DecodeError> {
        let keys: Vec<CFStringRef> =
            unsafe { vec![kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder] };
        let values: Vec<CFBoolean> = vec![CFBoolean::true_value()];

        let decoder_specification = unsafe {
            CFDictionaryCreate(
                std::ptr::null(),
                std::mem::transmute(keys.as_ptr()),
                std::mem::transmute(values.as_ptr()),
                keys.len().to_CFIndex(),
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            )
        };

        let format_description = unsafe {
            let mut format_ref = std::mem::MaybeUninit::<CMVideoFormatDescriptionRef>::uninit();

            let parameter_set_sizes = vec![vps_slice.len(), sps_slice.len(), pps_slice.len()];
            let parameter_sets = vec![vps_slice.as_ptr(), sps_slice.as_ptr(), pps_slice.as_ptr()];

            CMVideoFormatDescriptionCreateFromHEVCParameterSets(
                std::ptr::null(),     // Allocator
                parameter_sets.len(), // parameter set count
                parameter_sets.as_ptr(),
                parameter_set_sizes.as_ptr(),
                4,                                                      // NAL unit header length
                std::ptr::null(),                                       // extensions
                format_ref.as_mut_ptr() as CMVideoFormatDescriptionRef, // Format ref out
            );

            format_ref.assume_init()
        };

        // https://github.com/peter-iakovlev/TelegramUI/blob/e8b193443d1b84f00390138a82c44ebfcceb496a/TelegramUI/FFMpegMediaFrameSourceContextHelpers.swift#L67-L92
        // https://stackoverflow.com/questions/29525000/how-to-use-videotoolbox-to-decompress-h-264-video-stream/29525001#29525001

        // Create the decoder
        let mut decompression_ref = std::mem::MaybeUninit::<VTDecompressionSessionRef>::uninit();

        let callback_record = VTDecompressionOutputCallbackRecord {
            decompression_output_callback: Some(decode_callback),
            decompression_output_ref_con: std::ptr::null_mut(),
        };

        // Specify attributes for the destination image buffer.
        let dst_image_dictionary = unsafe {
            let format_type = kCVPixelFormatType_32BGRA;
            let format_type_ptr: *const u32 = &format_type;
            let pixel_format = CFNumberCreate(
                std::ptr::null(),
                kCFNumberSInt32Type,
                format_type_ptr as *const c_void,
            );

            let empty_dictionary = CFDictionaryCreate(
                std::ptr::null(),
                std::ptr::null(),
                std::ptr::null(),
                0,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            );

            let dst_image_dict = CFDictionaryCreateMutable(
                std::ptr::null(),
                2,
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            );

            CFDictionarySetValue(
                dst_image_dict,
                kCVPixelBufferPixelFormatTypeKey as *const c_void,
                pixel_format as *const c_void,
            );
            CFDictionarySetValue(
                dst_image_dict,
                kCVPixelBufferIOSurfacePropertiesKey as *const c_void,
                empty_dictionary as *const c_void,
            );

            dst_image_dict
        };

        let create_status = unsafe {
            VTDecompressionSessionCreate(
                std::ptr::null(),                                            // Allocator
                format_description,                                          // Format Description
                decoder_specification, // Decoder specification,
                dst_image_dictionary,  // Dest image buffer attributes
                &callback_record, // Output callback, pass NULL if you're using VTDecompressionSessionDecodeFrameWithOutputHandler
                decompression_ref.as_mut_ptr() as VTDecompressionSessionRef, // Decompression session out
            )
        };

        if create_status != 0 {
            println!("Failed to create VT Compression Session: {}", create_status);
            return Err(DecodeError::InitializationError(create_status));
        }

        let decompression_session = unsafe { decompression_ref.assume_init() };

        self.decode_session = Some(decompression_session);
        self.format_description = Some(format_description);

        Ok(())
    }

    fn decode(&mut self, src: &[u8], dst: &mut [u8]) -> Result<(), DecodeError> {
        let nal_iter = NalIterator::new(src);

        let frame_data = if let Some(_decode_session) = self.decode_session {
            let mut p_slice: Option<&[u8]> = None;

            for nal in nal_iter {
                println!("NAL Type: {:?}", nal.nal_type);

                if nal.nal_type == NalType::CodedSliceTrailR
                    || nal.nal_type == NalType::CodedSliceIdrNLp
                    || nal.nal_type == NalType::CodedSliceCra
                    || nal.nal_type == NalType::CodedSliceIdrWRadl
                {
                    p_slice = Some(nal.data);
                }
            }
            // If we have a decode session, look for P frames.
            // TODO - Loop through NAL units and assign a P frame to frame_data.
            p_slice.ok_or(DecodeError::MissingPFrame)?
        } else {
            // If we don't have a decode session, we need VPS, SPS, and PPS
            // NAL Units, along with an I Frame NAL Unit (IDR).
            let mut vps_slice: Option<&[u8]> = None;
            let mut sps_slice: Option<&[u8]> = None;
            let mut pps_slice: Option<&[u8]> = None;
            let mut idr_slice: Option<&[u8]> = None;

            for nal in nal_iter {
                println!("NAL Type: {:?}", nal.nal_type);
                if nal.nal_type == NalType::Vps {
                    vps_slice = Some(nal.data);
                }

                if nal.nal_type == NalType::Sps {
                    sps_slice = Some(nal.data);
                }

                if nal.nal_type == NalType::Pps {
                    pps_slice = Some(nal.data);
                }

                if nal.nal_type == NalType::CodedSliceIdrNLp
                    || nal.nal_type == NalType::CodedSliceIdrWRadl
                {
                    idr_slice = Some(nal.data);
                }
            }

            let vps_slice = vps_slice.ok_or(DecodeError::MissingVpsNalUnit)?;
            let sps_slice = sps_slice.ok_or(DecodeError::MissingSpsNalUnit)?;
            let pps_slice = pps_slice.ok_or(DecodeError::MissingPpsNalUnit)?;

            // Recreate
            self.recreate_decoder(vps_slice, sps_slice, pps_slice)?;

            idr_slice.ok_or(DecodeError::MissingIFrame)?
        };

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
                return Err(DecodeError::BlockBufferCreationError(status));
            }

            block_buffer_out.assume_init()
        };

        let sample_buffer = unsafe {
            let sample_size = frame_data.len();
            let mut sample_buffer_out = std::mem::MaybeUninit::<CMSampleBufferRef>::uninit();

            let status = CMSampleBufferCreate(
                std::ptr::null(),                                                   // Allocator
                block_buffer,                                                       // Data
                true,                                                               // Data Ready
                None,                 // Make data ready callback
                std::ptr::null_mut(), // Make data ready callback ref con
                self.format_description.expect("Should have a format description"), // Format description
                1,                                                                  // Num samples
                0,                                                   // Num sample timing entries
                std::ptr::null(),                                    // Sample timing array
                1,                                                   // Num sample timing entries
                &sample_size,                                        // Sample size
                sample_buffer_out.as_mut_ptr() as CMSampleBufferRef, // Sample buffer out
            );

            if status != 0 {
                println!("Error creating CMSampleBuffer");
                return Err(DecodeError::SampleBufferCreationError(status));
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

        // TODO - allocate in a Box.
        let mut dst_buffer = DstBuffer { data: dst.as_mut_ptr(), len: dst.len(), written_size: 0 };

        unsafe {
            VTDecompressionSessionDecodeFrame(
                self.decode_session.unwrap(),
                sample_buffer,
                0,                                                // Decode flags
                &mut dst_buffer as *mut DstBuffer as *mut c_void, // User data
                std::ptr::null_mut(),                             // Info flags out
            );
        }

        let _ = unsafe {
            VTDecompressionSessionWaitForAsynchronousFrames(self.decode_session.unwrap())
        };

        Ok(())
    }
}

extern "C" fn decode_callback(
    _output_callback_ref_con: *mut c_void,
    source_frame_ref_con: *mut c_void,
    status: OSStatus,
    _info_flags: VTDecodeInfoFlags,
    image_buffer: CVImageBufferRef,
    _presentation_timestamp: CMTime,
    _presentation_duration: CMTime,
) {
    println!("decode_callback");
    println!("Status: {}", status);

    unsafe {
        if let Some(dst_buffer) = (source_frame_ref_con as *mut DstBuffer).as_mut() {
            println!(
                "We have a frame to write to, it has dimensions {}x{}",
                CVPixelBufferGetWidth(image_buffer),
                CVPixelBufferGetHeight(image_buffer)
            );
            println!("Buffer encoded size is {:?}", CVImageBufferGetEncodedSize(image_buffer));
            println!("Buffer display size is {:?}", CVImageBufferGetDisplaySize(image_buffer));
            println!("Data size is {:?}", CVPixelBufferGetDataSize(image_buffer));
            println!("The buffer is planar: {}", CVPixelBufferIsPlanar(image_buffer));
            println!("The buffer has {} planes", CVPixelBufferGetPlaneCount(image_buffer));
            println!(
                "The pixel format type is 0x{:x}",
                CVPixelBufferGetPixelFormatType(image_buffer)
            );

            // Lock the buffer and copy it to our output buffer.
            let _ = CVPixelBufferLockBaseAddress(image_buffer, kCVPixelBufferLock_ReadOnly);

            let plane = 0;
            let plane_base_address = CVPixelBufferGetBaseAddressOfPlane(image_buffer, plane);
            let bytes_per_row = CVPixelBufferGetBytesPerRowOfPlane(image_buffer, plane);
            let num_rows = CVPixelBufferGetHeightOfPlane(image_buffer, plane);
            let src_len = bytes_per_row * num_rows;

            let src_slice: &[u8] =
                std::slice::from_raw_parts(plane_base_address as *const u8, src_len);
            let dst_slice = std::slice::from_raw_parts_mut(dst_buffer.data, dst_buffer.len);

            dst_slice[..src_len].copy_from_slice(src_slice);

            let _ = CVPixelBufferUnlockBaseAddress(image_buffer, kCVPixelBufferLock_ReadOnly);
        }
    }
}

#[allow(unused)]
struct DstBuffer {
    data: *mut u8,
    len: usize,
    written_size: usize,
}
