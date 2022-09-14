use core_foundation::{
    base::{CFIndexConvertible, OSStatus},
    boolean::CFBoolean,
    dictionary::{
        kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate,
    },
    string::CFStringRef,
};
use std::{convert::TryInto, os::raw::c_void};
use video_toolbox_sys::{
    kCMVideoCodecType_HEVC, kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder,
    CMBlockBufferCopyDataBytes, CMFormatDescriptionRef, CMSampleBufferGetDataBuffer,
    CMSampleBufferGetFormatDescription, CMSampleBufferGetTotalSampleSize, CMSampleBufferRef,
    CMTime, CMVideoFormatDescriptionGetHEVCParameterSetAtIndex, CVPixelBufferCreateWithBytes,
    CVPixelBufferRef, VTCompressionSessionCompleteFrames, VTCompressionSessionCreate,
    VTCompressionSessionEncodeFrame, VTCompressionSessionRef, VTEncodeInfoFlags,
};

extern "C" fn encode_callback(
    _output_callback_ref_con: *mut std::os::raw::c_void,
    source_frame_ref_con: *mut std::os::raw::c_void,
    _status: OSStatus,
    _info_flags: VTEncodeInfoFlags,
    sample_buffer: CMSampleBufferRef,
) {
    // Returns the total size in bytes of sample data in a CMSampleBuffer.
    let data_length = unsafe { CMSampleBufferGetTotalSampleSize(sample_buffer) };
    let data_buffer = unsafe { CMSampleBufferGetDataBuffer(sample_buffer) };
    let format = unsafe { CMSampleBufferGetFormatDescription(sample_buffer) };

    let vps = get_hevc_param(format, HevcParam::Vps).unwrap();
    let sps = get_hevc_param(format, HevcParam::Sps).unwrap();
    let pps = get_hevc_param(format, HevcParam::Pps).unwrap();

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
    output.extend_from_slice(HEADER);
    output.extend_from_slice(&vps);

    output.extend_from_slice(HEADER);
    output.extend_from_slice(&sps);

    output.extend_from_slice(HEADER);
    output.extend_from_slice(&pps);

    let mut buffer_offset = 0;

    while buffer_offset < (hevc_data.len() - HEADER.len()) {
        let mut nal_len = u32::from_ne_bytes([
            hevc_data[buffer_offset],
            hevc_data[(buffer_offset + 1)],
            hevc_data[(buffer_offset + 2)],
            hevc_data[(buffer_offset + 3)],
        ]);
        nal_len = u32::from_be(nal_len);

        output.extend_from_slice(HEADER);
        let hevc_offset = buffer_offset + HEADER.len();
        output.extend_from_slice(&hevc_data[hevc_offset..(hevc_offset + nal_len as usize)]);

        buffer_offset += HEADER.len();
        buffer_offset += nal_len as usize;
    }

    std::mem::forget(vps);
    std::mem::forget(sps);
    std::mem::forget(pps);

    std::fs::write("out.hevc", &output).unwrap();

    unsafe {
        if let Some(custom_val) = (source_frame_ref_con as *mut u32).as_mut() {
            *custom_val = 37;
        }
    }
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

    if status == 0 {
        unsafe {
            let vec = Vec::from_raw_parts(param_set_ptr as *mut _, param_set_size, param_set_size);
            Some(vec)
        }
    } else {
        None
    }
}

fn main() {
    let frame_width = 1280usize;
    let frame_height = 720usize;

    let mut compression_ref = std::mem::MaybeUninit::<VTCompressionSessionRef>::uninit();

    let keys: Vec<CFStringRef> =
        unsafe { vec![kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder] };
    let values: Vec<CFBoolean> = vec![CFBoolean::true_value()];

    let encoder_specification = unsafe {
        CFDictionaryCreate(
            std::ptr::null(),
            std::mem::transmute(keys.as_ptr()),
            std::mem::transmute(values.as_ptr()),
            keys.len().to_CFIndex().try_into().unwrap(),
            &kCFTypeDictionaryKeyCallBacks,
            &kCFTypeDictionaryValueCallBacks,
        )
    };

    // Create the encoder
    let create_status = unsafe {
        VTCompressionSessionCreate(
            std::ptr::null(),       // Allocator
            frame_width as i32,     // Width
            frame_height as i32,    // Height
            kCMVideoCodecType_HEVC, // Codec type
            encoder_specification,  // Encoder specification,
            std::ptr::null(),       // Src pixel buffer attributes
            std::ptr::null(),       // Compressed data allocator
            Some(encode_callback), // Output callback, pass NULL if you're using VTCompressionSessionEncodeFrameWithOutputHandler
            std::ptr::null_mut(),  // Client-defined reference value for the output callback
            compression_ref.as_mut_ptr() as VTCompressionSessionRef,
        )
    };

    if create_status != 0 {
        return;
    }

    let compression_session = unsafe { compression_ref.assume_init() };

    // Create the frame to encode
    // let mut frame_data = vec![0u8; (frame_width * frame_height * 4) as usize];
    let frame_data = make_image_frame(frame_width, frame_height);

    let mut pixel_buffer_ref = std::mem::MaybeUninit::<CVPixelBufferRef>::uninit();
    let k_cvpixel_format_type_32_argb = 0x00000020; // TODO(bschwind) - get this from CoreVideo
    let pixel_buffer_create_status = unsafe {
        CVPixelBufferCreateWithBytes(
            std::ptr::null(),
            frame_width as usize,
            frame_height as usize,
            k_cvpixel_format_type_32_argb,
            frame_data.as_ptr() as *mut c_void,
            (4 * frame_width) as usize, // bytes per row
            None,
            std::ptr::null_mut(),
            std::ptr::null(),
            pixel_buffer_ref.as_mut_ptr() as *mut CVPixelBufferRef,
        )
    };

    if pixel_buffer_create_status != 0 {
        return;
    }

    let pixel_buffer = unsafe { pixel_buffer_ref.assume_init() };

    let frame_time = CMTime { value: 0i64, timescale: 1i32, flags: 0u32, epoch: 0i64 };

    let invalid_duration = CMTime { value: 0i64, timescale: 0i32, flags: 0u32, epoch: 0i64 };

    let mut custom_val = 0u32;
    // Encode the frame
    let _encode_status = unsafe {
        VTCompressionSessionEncodeFrame(
            compression_session,
            pixel_buffer,
            frame_time,                                 // Presentation timestamp
            invalid_duration,                           // Frame duration
            std::ptr::null(),                           // Frame Properties
            &mut custom_val as *mut u32 as *mut c_void, // Source frame ref con
            std::ptr::null_mut(),                       // Info flags out
        );
    };

    // Wait for the encode to finish.
    let _ = unsafe {
        VTCompressionSessionCompleteFrames(compression_session, invalid_duration);
    };
}

fn make_image_frame(width: usize, height: usize) -> Vec<u8> {
    let mut frame = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let pixel_offset = (y * width * 4) + (x * 4);

            let width_factor = x as f32 / width as f32;
            let height_factor = y as f32 / height as f32;

            frame[pixel_offset] = 255; // Alpha
            frame[pixel_offset + 1] = (width_factor * 255.0) as u8; // Red
            frame[pixel_offset + 2] = 255; // Green
            frame[pixel_offset + 3] = (height_factor * 255.0) as u8; // Blue
        }
    }

    frame
}
