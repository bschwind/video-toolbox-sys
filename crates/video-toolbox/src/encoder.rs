use core::ffi::c_void;
use core_foundation::{
    array::{CFArrayGetCount, CFArrayGetValueAtIndex},
    base::{CFIndexConvertible, OSStatus},
    boolean::CFBoolean,
    dictionary::{
        kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate,
        CFDictionaryGetValueIfPresent, CFDictionaryRef,
    },
    number::{CFBooleanGetValue, CFBooleanRef},
    string::CFStringRef,
};
use thiserror::Error;
use video_toolbox_sys::{
    kCMSampleAttachmentKey_NotSync, kCMVideoCodecType_HEVC,
    kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder,
    CMBlockBufferCopyDataBytes, CMFormatDescriptionRef, CMSampleBufferGetDataBuffer,
    CMSampleBufferGetFormatDescription, CMSampleBufferGetSampleAttachmentsArray,
    CMSampleBufferGetTotalSampleSize, CMSampleBufferIsValid, CMSampleBufferRef, CMTime,
    CMVideoFormatDescriptionGetHEVCParameterSetAtIndex, CVPixelBufferCreateWithBytes,
    CVPixelBufferRef, OpaqueVTCompressionSession, VTCompressionSessionCompleteFrames,
    VTCompressionSessionCreate, VTCompressionSessionEncodeFrame, VTCompressionSessionRef,
    VTEncodeInfoFlags,
};

#[derive(Debug, Error)]
pub enum EncodeError {
    #[error("Initialization Error: {0}")]
    InitializationError(i32),

    #[error("Pixel Buffer Creation Error: {0}")]
    PixelBufferCreationError(i32),
}

pub struct Encoder {
    width: u32,
    height: u32,
    encode_session: *mut OpaqueVTCompressionSession,
}

unsafe impl Send for Encoder {}

impl Drop for Encoder {
    fn drop(&mut self) {
        // TODO - call VTCompressionSessionInvalidate
    }
}

impl Encoder {
    pub fn new(width: u32, height: u32) -> Result<Self, EncodeError> {
        let mut encode_ref = std::mem::MaybeUninit::<VTCompressionSessionRef>::uninit();

        // Require hardware-accelerated encoding.
        let keys: Vec<CFStringRef> =
            unsafe { vec![kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder] };

        let values: Vec<CFBoolean> = vec![CFBoolean::true_value()];

        let encoder_specification = unsafe {
            CFDictionaryCreate(
                std::ptr::null(),
                std::mem::transmute(keys.as_ptr()),
                std::mem::transmute(values.as_ptr()),
                keys.len().to_CFIndex(),
                &kCFTypeDictionaryKeyCallBacks,
                &kCFTypeDictionaryValueCallBacks,
            )
        };

        // Create the encoder
        let create_status = unsafe {
            VTCompressionSessionCreate(
                std::ptr::null(),       // Allocator
                width as i32,           // Width
                height as i32,          // Height
                kCMVideoCodecType_HEVC, // Codec type
                encoder_specification,  // Encoder specification,
                std::ptr::null(),       // Src pixel buffer attributes
                std::ptr::null(),       // Compressed data allocator
                Some(encode_callback), // Output callback, pass NULL if you're using VTCompressionSessionEncodeFrameWithOutputHandler
                std::ptr::null_mut(),  // Client-defined reference value for the output callback
                encode_ref.as_mut_ptr() as VTCompressionSessionRef,
            )
        };

        if create_status != 0 {
            println!("Failed to create VT Compression Session: {}", create_status);
            return Err(EncodeError::InitializationError(create_status));
        }

        let encode_session = unsafe { encode_ref.assume_init() };

        Ok(Self { width, height, encode_session })
    }

    pub fn width(&self) -> u32 {
        self.width
    }

    pub fn height(&self) -> u32 {
        self.height
    }

    /// Encodes an uncompressed video frame from `src` into `dst`.
    pub fn encode_blocking(&mut self, src: &[u8], dst: &mut [u8]) -> Result<usize, EncodeError> {
        let mut pixel_buffer_ref = std::mem::MaybeUninit::<CVPixelBufferRef>::uninit();
        let k_cvpixel_format_type_32_argb = 0x00000020; // TODO(bschwind) - get this from CoreVideo
        let pixel_buffer_create_status = unsafe {
            CVPixelBufferCreateWithBytes(
                std::ptr::null(),
                self.width as usize,
                self.height as usize,
                k_cvpixel_format_type_32_argb,
                src.as_ptr() as *mut c_void,
                (4 * self.width) as usize, // bytes per row
                None,
                std::ptr::null_mut(),
                std::ptr::null(),
                pixel_buffer_ref.as_mut_ptr() as *mut CVPixelBufferRef,
            )
        };

        if pixel_buffer_create_status != 0 {
            println!("Failed to create Pixel Buffer: {}", pixel_buffer_create_status);
            return Err(EncodeError::PixelBufferCreationError(pixel_buffer_create_status));
        }

        let pixel_buffer = unsafe { pixel_buffer_ref.assume_init() };

        println!("Got a pixel buffer, good to go!");

        let frame_time = CMTime { value: 0i64, timescale: 1i32, flags: 0u32, epoch: 0i64 };

        let invalid_duration = CMTime { value: 0i64, timescale: 0i32, flags: 0u32, epoch: 0i64 };

        // TODO - allocate in a Box.
        let mut dst_buffer = DstBuffer { data: dst.as_mut_ptr(), len: dst.len(), written_size: 0 };

        // Encode the frame
        let encode_status = unsafe {
            VTCompressionSessionEncodeFrame(
                self.encode_session,
                pixel_buffer,
                frame_time,                                       // Presentation timestamp
                invalid_duration,                                 // Frame duration
                std::ptr::null(),                                 // Frame Properties
                &mut dst_buffer as *mut DstBuffer as *mut c_void, // Source frame ref con
                std::ptr::null_mut(),                             // Info flags out
            )
        };

        println!("Encode status: {:?}", encode_status);

        // Wait for the encode to finish.
        let _ =
            unsafe { VTCompressionSessionCompleteFrames(self.encode_session, invalid_duration) };

        let written_size = dst_buffer.written_size;

        Ok(written_size)
    }
}

extern "C" fn encode_callback(
    _output_callback_ref_con: *mut std::os::raw::c_void,
    source_frame_ref_con: *mut std::os::raw::c_void,
    status: OSStatus,
    _info_flags: VTEncodeInfoFlags,
    sample_buffer: CMSampleBufferRef,
) {
    println!("encode_callback");

    let attachments = unsafe { CMSampleBufferGetSampleAttachmentsArray(sample_buffer, false) };
    let is_iframe = unsafe {
        if CFArrayGetCount(attachments) > 0 {
            let mut is_iframe = std::mem::MaybeUninit::<CFBooleanRef>::uninit();

            let attachment_dictionary = CFArrayGetValueAtIndex(attachments, 0) as CFDictionaryRef;
            let value_present = CFDictionaryGetValueIfPresent(
                attachment_dictionary,
                kCMSampleAttachmentKey_NotSync as *const c_void,
                is_iframe.as_mut_ptr() as *mut *const c_void,
            );

            let is_iframe = is_iframe.assume_init();
            value_present == 0 || !CFBooleanGetValue(is_iframe)
        } else {
            false
        }
    };

    println!("Status: {}", status);
    println!("Is I-frame: {}", is_iframe);

    println!("Valid buffer: {}", unsafe { CMSampleBufferIsValid(sample_buffer) });
    // Returns the total size in bytes of sample data in a CMSampleBuffer.
    let data_length = unsafe { CMSampleBufferGetTotalSampleSize(sample_buffer) };
    println!("Total sample size: {}", data_length);

    let data_buffer = unsafe { CMSampleBufferGetDataBuffer(sample_buffer) };
    println!("Data buffer: {:?}", data_buffer);

    let format = unsafe { CMSampleBufferGetFormatDescription(sample_buffer) };

    let mut hevc_data = vec![0u8; data_length];

    let offset = 0;
    let _ = unsafe {
        CMBlockBufferCopyDataBytes(
            data_buffer,
            offset,
            data_length,
            hevc_data.as_mut_ptr() as *mut _,
        )
    };

    const HEADER: &[u8; 4] = &[0, 0, 0, 1];

    let mut output = vec![];

    if is_iframe {
        let vps = get_hevc_param(format, HevcParam::Vps).unwrap();
        let sps = get_hevc_param(format, HevcParam::Sps).unwrap();
        let pps = get_hevc_param(format, HevcParam::Pps).unwrap();

        output.extend_from_slice(HEADER);
        output.extend_from_slice(&vps);

        output.extend_from_slice(HEADER);
        output.extend_from_slice(&sps);

        output.extend_from_slice(HEADER);
        output.extend_from_slice(&pps);

        std::mem::forget(vps);
        std::mem::forget(sps);
        std::mem::forget(pps);
    }

    let mut buffer_offset = 0;

    // VideoToolbox will prefix a 4-byte length value on each
    // NAL Unit.
    const LENGTH_PREFIX_SIZE: usize = 4;

    // Convert from AVCC format to Annex B format.
    // Find each NAL unit, strip the 4 byte length prefix, replace it
    // with the HEADER, and append the data to the output buffer.
    while buffer_offset < (hevc_data.len() - HEADER.len()) {
        let mut nal_len = u32::from_ne_bytes([
            hevc_data[buffer_offset],
            hevc_data[(buffer_offset + 1)],
            hevc_data[(buffer_offset + 2)],
            hevc_data[(buffer_offset + 3)],
        ]);
        nal_len = u32::from_be(nal_len);
        dbg!(nal_len);

        output.extend_from_slice(HEADER);
        let hevc_offset = buffer_offset + LENGTH_PREFIX_SIZE; // Replace length prefix with HEADER.
        output.extend_from_slice(&hevc_data[hevc_offset..(hevc_offset + nal_len as usize)]);

        buffer_offset += LENGTH_PREFIX_SIZE;
        buffer_offset += nal_len as usize;
    }

    unsafe {
        if let Some(dst_buffer) = (source_frame_ref_con as *mut DstBuffer).as_mut() {
            dst_buffer.written_size = output.len();

            let dst_slice = std::slice::from_raw_parts_mut(dst_buffer.data, dst_buffer.len);
            dst_slice[..output.len()].copy_from_slice(&output);
        }
    }

    dbg!(output.len());
}

struct DstBuffer {
    data: *mut u8,
    len: usize,
    written_size: usize,
}

#[derive(Debug)]
enum HevcParam {
    Vps,
    Sps,
    Pps,
}

impl HevcParam {
    fn index(&self) -> usize {
        match self {
            HevcParam::Vps => 0,
            HevcParam::Sps => 1,
            HevcParam::Pps => 2,
        }
    }
}

fn get_hevc_param(format: CMFormatDescriptionRef, param: HevcParam) -> Option<Vec<u8>> {
    let mut param_set_ptr: *const u8 = std::ptr::null_mut();
    let mut param_set_size: usize = 0;
    let mut param_set_count: usize = 0;
    let mut nal_unit_header_length: std::os::raw::c_int = 0;

    let status = unsafe {
        CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(
            format,
            param.index(),
            &mut param_set_ptr,
            &mut param_set_size,
            &mut param_set_count,
            &mut nal_unit_header_length,
        )
    };

    println!(
        "{:?} - size: {}, count: {}, NAL header len: {:?}",
        param, param_set_size, param_set_count, nal_unit_header_length
    );

    if status == 0 {
        unsafe {
            let vec = Vec::from_raw_parts(param_set_ptr as *mut _, param_set_size, param_set_size);
            println!("{:?}", vec);
            Some(vec)
        }
    } else {
        None
    }
}
