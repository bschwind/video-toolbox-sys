use core_foundation::base::CFIndexConvertible;
use std::convert::TryInto;
use video_toolbox_sys::{
    kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, kCMVideoCodecType_HEVC,
    CFDictionaryCreate,
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

#[link(name = "VideoToolBox", kind = "framework")]
extern "C" {
    pub static kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder: CFStringRef;
}

fn main() {
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

    println!(
        "Fourcc compare: Mine: {}, Theirs: {}",
        fourcc('h' as u8, 'v' as u8, 'c' as u8, '1' as u8),
        kCMVideoCodecType_HEVC
    );

    let create_status = unsafe {
        VTCompressionSessionCreate(
            std::ptr::null(), // Allocator
            1280,             // Width
            720,              // Height
            kCMVideoCodecType_HEVC,
            // encoder_specification, // Encoder Specification
            std::ptr::null(),
            std::ptr::null(),     // Src pixel buffer attributes
            std::ptr::null(),     // Compressed data allocator
            None, // Output callback, pass NULL if you're using  VTCompressionSessionEncodeFrame
            std::ptr::null_mut(), // Client-defined reference value for the output callback
            compression_ref.as_mut_ptr() as *mut VTCompressionSessionRef,
        )
    };

    if create_status != 0 {
        println!("Failed to create VT Compression Session: {}", create_status);
        return;
    }
}
