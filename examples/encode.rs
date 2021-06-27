use core_foundation::base::{CFIndexConvertible, OSStatus};
use std::{
    convert::TryInto,
    os::raw::{c_int, c_void},
};
use video_toolbox_sys::{
    kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, kCMVideoCodecType_HEVC,
    CFDictionaryCreate, CMBlockBufferCopyDataBytes, CMSampleBufferGetDataBuffer,
    CMSampleBufferGetTotalSampleSize, CMSampleBufferIsValid, CMSampleBufferRef, CMTime,
    CVPixelBufferCreateWithBytes, CVPixelBufferRef, FourCharCode, VTCompressionSessionEncodeFrame,
    VTEncodeInfoFlags,
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

#[link(name = "CoreMedia", kind = "framework")]
extern "C" {}

unsafe extern "C" fn encode_callback(
    outputCallbackRefCon: *mut ::std::os::raw::c_void,
    sourceFrameRefCon: *mut ::std::os::raw::c_void,
    status: OSStatus,
    infoFlags: VTEncodeInfoFlags,
    sampleBuffer: CMSampleBufferRef,
) {
    println!("encode_callback");

    // Returns the total size in bytes of sample data in a CMSampleBuffer.
    println!("Valid buffer: {}", CMSampleBufferIsValid(sampleBuffer));
    let data_length = CMSampleBufferGetTotalSampleSize(sampleBuffer);
    println!("Total sample size: {}", data_length);

    let data_buffer = CMSampleBufferGetDataBuffer(sampleBuffer);
    println!("Data buffer: {:?}", data_buffer);

    let mut dest = vec![0u8; data_length];
    let offset = 0;
    let _ =
        CMBlockBufferCopyDataBytes(data_buffer, offset, data_length, dest.as_mut_ptr() as *mut _);

    dbg!(dest.len());
}

fn main() {
    let frame_width = 1280usize;
    let frame_height = 720usize;

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
            std::ptr::null(),    // Allocator
            frame_width as i32,  // Width
            frame_height as i32, // Height
            kCMVideoCodecType_HEVC,
            // encoder_specification, // Encoder Specification
            std::ptr::null(),
            std::ptr::null(),      // Src pixel buffer attributes
            std::ptr::null(),      // Compressed data allocator
            Some(encode_callback), // Output callback, pass NULL if you're using VTCompressionSessionEncodeFrameWithOutputHandler
            std::ptr::null_mut(),  // Client-defined reference value for the output callback
            compression_ref.as_mut_ptr() as *mut VTCompressionSessionRef,
        )
    };

    if create_status != 0 {
        println!("Failed to create VT Compression Session: {}", create_status);
        return;
    }

    let compression_session = unsafe { compression_ref.assume_init() };

    // Create the frame to encode
    // let mut frame_data = vec![0u8; (frame_width * frame_height * 4) as usize];
    let frame_data = make_image_frame(frame_width, frame_height);

    println!("Uncompressed size: {}", frame_data.len());

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

    let frame_time = CMTime { value: 0i64, timescale: 1i32, flags: 0u32, epoch: 0i64 };

    let invalid_duration = CMTime { value: 0i64, timescale: 0i32, flags: 0u32, epoch: 0i64 };

    // Encode the frame
    let encode_status = unsafe {
        VTCompressionSessionEncodeFrame(
            compression_session,
            pixel_buffer,
            frame_time,           // Presentation timestamp
            invalid_duration,     // Frame duration
            std::ptr::null(),     // Frame Properties
            std::ptr::null_mut(), // Source frame ref con
            std::ptr::null_mut(), // Info flags out
        );
    };

    std::thread::sleep(std::time::Duration::from_secs(2));
}

fn make_image_frame(width: usize, height: usize) -> Vec<u8> {
    let mut frame = vec![0u8; width * height * 4];

    for y in 0..height {
        for x in 0..width {
            let pixel_offset = (y * width * 4) + (x * 4);

            let width_factor = x as f32 / width as f32;
            let height_factor = y as f32 / height as f32;

            frame[pixel_offset] = (width_factor * 255.0) as u8; // Red
            frame[pixel_offset + 1] = 255; // Green
            frame[pixel_offset + 2] = (height_factor * 255.0) as u8; // Blue
            frame[pixel_offset + 3] = 255; // Alpha
        }
    }

    frame
}
