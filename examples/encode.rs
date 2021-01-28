use core_foundation::base::CFIndexConvertible;
use std::{convert::TryInto, os::raw::c_void};
use video_toolbox_sys::{
    kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, kCMVideoCodecType_HEVC,
    CFDictionaryCreate, CVPixelBufferCreateWithBytes, CVPixelBufferRef, FourCharCode,
    VTCompressionSessionEncodeFrameWithOutputHandler,
};
// use core_foundation::dictionary::CFDictionaryCreate;
use core_foundation::{
    boolean::CFBoolean,
    string::{CFString, CFStringRef},
};
use video_toolbox_sys::{VTCompressionSessionCreate, VTCompressionSessionRef};

const fn fourcc(a: u8, b: u8, c: u8, d: u8) -> u32 {
    ((a as u32) << 24) | ((b as u32) << 16) | ((c as u32) << 8) | d as u32
}

#[link(name = "VideoToolbox", kind = "framework")]
extern "C" {
    pub static kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder: CFStringRef;
}

#[link(name = "CoreVideo", kind = "framework")]
extern "C" {}

fn main() {
    let frame_width = 1280;
    let frame_height = 720;

    // let mut compression_ref = OpaqueVTCompressionSession { _unused: [0; 0] };
    let mut compression_ref = std::mem::MaybeUninit::<VTCompressionSessionRef>::uninit();

    // let keys: Vec<CFStringRef> = unsafe { vec![kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder] };
    // let values: Vec<CFBoolean> = vec![CFBoolean::true_value()];

    // let encoder_specification = unsafe { CFDictionaryCreate(
    //     std::ptr::null(),
    //     std::mem::transmute(keys.as_ptr()),
    //     std::mem::transmute(values.as_ptr()),
    //     keys.len().to_CFIndex().try_into().unwrap(),
    //     &kCFTypeDictionaryKeyCallBacks,
    //     &kCFTypeDictionaryValueCallBacks,
    // ) };

    // Create the encoder
    let create_status = unsafe {
        VTCompressionSessionCreate(
            std::ptr::null(), // Allocator
            frame_width,      // Width
            frame_height,     // Height
            kCMVideoCodecType_HEVC,
            // encoder_specification, // Encoder Specification
            std::ptr::null(),
            std::ptr::null(),     // Src pixel buffer attributes
            std::ptr::null(),     // Compressed data allocator
            None, // Output callback, pass NULL if you're using VTCompressionSessionEncodeFrameWithOutputHandler
            std::ptr::null_mut(), // Client-defined reference value for the output callback
            compression_ref.as_mut_ptr() as *mut VTCompressionSessionRef,
        )
    };

    if create_status != 0 {
        println!("Failed to create VT Compression Session: {}", create_status);
        return;
    }

    let compression_session = unsafe { compression_ref.assume_init() };

    // Create the frame to encode
    let mut frame_data = vec![0u8; (frame_width * frame_height * 4) as usize];

    let mut pixel_buffer_ref = std::mem::MaybeUninit::<CVPixelBufferRef>::uninit();
    let kCVPixelFormatType_32ARGB = 0x00000020; // TODO(bschwind) - get this from CoreVideo
    let pixel_buffer_create_status = unsafe {
        CVPixelBufferCreateWithBytes(
            std::ptr::null(),
            frame_width as usize,
            frame_height as usize,
            kCVPixelFormatType_32ARGB,
            frame_data.as_ptr() as *mut c_void,
            (4 * frame_width) as usize, // bytes per row
            None,
            std::ptr::null_mut(),
            std::ptr::null(),
            pixel_buffer_ref.as_mut_ptr() as *mut CVPixelBufferRef,
        )
    };

    if pixel_buffer_create_status != 0 {
        println!("Failed to create Pixel Buffer: {}", pixel_buffer_create_status);
        return;
    }

    let pixel_buffer = unsafe { pixel_buffer_ref.assume_init() };

    println!("Got a pixel buffer, good to go!");

    // Encode the frame
    // let encode_status = unsafe {
    //     VTCompressionSessionEncodeFrameWithOutputHandler(
    //         compression_session,
    //         (),
    //         (),
    //         (),
    //         (),
    //         (),
    //         (),
    //     );
    // };
}
