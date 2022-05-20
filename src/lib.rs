#![cfg(any(target_os = "macos", target_os = "ios"))]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use core_foundation::{
    base::{Boolean, CFAllocatorRef, OSStatus},
    dictionary::CFDictionaryRef,
    string::CFStringRef,
};
use std::os::raw::{c_int, c_void};

#[repr(C)]
pub struct OpaqueVTCompressionSession {
    _data: [u8; 0],
}

#[repr(C)]
pub struct OpaqueVTDecompressionSession {
    _data: [u8; 0],
}

pub type VTCompressionSessionRef = *mut OpaqueVTCompressionSession;
pub type VTDecompressionSessionRef = *mut OpaqueVTDecompressionSession;
pub type VTEncodeInfoFlags = u32;
pub type VTDecodeInfoFlags = u32;

// CoreMedia Types
pub type FourCharCode = u32;
pub type OSType = FourCharCode;
pub type CMVideoCodecType = FourCharCode;

#[repr(C)]
pub struct OpaqueCMSampleBuffer {
    _data: [u8; 0],
}

pub type CMSampleBufferRef = *mut OpaqueCMSampleBuffer;

#[repr(C)]
pub struct OpaqueCMBlockBuffer {
    _data: [u8; 0],
}

pub type CMBlockBufferRef = *mut OpaqueCMBlockBuffer;

#[repr(C)]
pub struct CMFormatDescription {
    _data: [u8; 0],
}

pub type CMFormatDescriptionRef = *mut CMFormatDescription;
pub type CMVideoFormatDescriptionRef = CMFormatDescriptionRef;

#[repr(C)]
#[derive(Debug, Copy, Clone)]
pub struct CMTime {
    pub value: i64,
    pub timescale: i32,
    pub flags: u32,
    pub epoch: i64,
}

const fn fourcc(data: &[u8; 4]) -> u32 {
    ((data[0] as u32) << 24) | ((data[1] as u32) << 16) | ((data[2] as u32) << 8) | data[3] as u32
}

pub const kCMVideoCodecType_HEVC: CMVideoCodecType = fourcc(b"hvc1");
// TODO - Define all others listed here:
// https://developer.apple.com/documentation/coremedia/cmvideocodectype?language=objc

// CoreVideo Types
pub type CVReturn = i32;

#[repr(C)]
pub struct CVBuffer {
    _data: [u8; 0],
}

pub type CVBufferRef = *mut CVBuffer;
pub type CVImageBufferRef = CVBufferRef;
pub type CVPixelBufferRef = CVImageBufferRef;

pub type CVPixelBufferReleaseBytesCallback =
    extern "C" fn(release_ref_con: *mut c_void, base_address: *const c_void);

// Callback Types
pub type VTCompressionOutputCallback = extern "C" fn(
    output_callback_ref_con: *mut c_void,
    source_frame_ref_con: *mut c_void,
    status: OSStatus,
    info_flags: VTEncodeInfoFlags,
    sample_buffer: CMSampleBufferRef,
);

pub type VTDecompressionOutputCallback = extern "C" fn(
    output_callback_ref_con: *mut c_void,
    source_frame_ref_con: *mut c_void,
    status: OSStatus,
    info_flags: VTDecodeInfoFlags,
    image_buffer: CVImageBufferRef,
    presentation_timestamp: CMTime,
    presentation_duration: CMTime,
);

// Encoding
#[link(name = "VideoToolbox", kind = "framework")]
extern "C" {
    // Encoding
    pub static kVTVideoEncoderSpecification_RequireHardwareAcceleratedVideoEncoder: CFStringRef;

    pub fn VTCompressionSessionCreate(
        allocator: CFAllocatorRef,
        width: i32,
        height: i32,
        codec_type: CMVideoCodecType,
        encoder_specification: CFDictionaryRef,
        source_image_buffer_attributes: CFDictionaryRef,
        compressed_data_allocator: CFAllocatorRef,
        output_callback: Option<VTCompressionOutputCallback>,
        output_callback_ref_con: *mut c_void,
        compression_session_out: VTCompressionSessionRef,
    ) -> OSStatus;

    pub fn VTCompressionSessionEncodeFrame(
        session: VTCompressionSessionRef,
        image_buffer: CVImageBufferRef,
        presentation_timestamp: CMTime,
        duration: CMTime,
        frame_properties: CFDictionaryRef,
        source_frame_ref_con: *mut c_void,
        info_flags_out: *mut VTEncodeInfoFlags,
    ) -> OSStatus;

    pub fn VTCompressionSessionCompleteFrames(
        session: VTCompressionSessionRef,
        complete_until_presentation_timestamp: CMTime,
    ) -> OSStatus;

    // Decoding
    pub static kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder: CFStringRef;

    pub fn VTDecompressionSessionCreate(
        allocator: CFAllocatorRef,
        video_format_description: CMVideoFormatDescriptionRef,
        video_decoder_specification: CFDictionaryRef,
        destination_image_buffer_attributes: CFDictionaryRef,
        output_callback: Option<VTDecompressionOutputCallback>,
        decompression_session_out: VTDecompressionSessionRef,
    ) -> OSStatus;
}

// CoreMedia
#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    pub fn CMSampleBufferIsValid(sample_buffer: CMSampleBufferRef) -> Boolean;
    pub fn CMSampleBufferGetTotalSampleSize(sample_buffer: CMSampleBufferRef) -> usize;
    pub fn CMSampleBufferGetDataBuffer(sample_buffer: CMSampleBufferRef) -> CMBlockBufferRef;
    pub fn CMSampleBufferGetFormatDescription(
        sample_buffer: CMSampleBufferRef,
    ) -> CMFormatDescriptionRef;
    pub fn CMVideoFormatDescriptionGetHEVCParameterSetAtIndex(
        video_desc: CMFormatDescriptionRef,
        parameter_set_index: usize,
        parameters_set_pointer_out: *mut *const u8,
        parameter_set_size_out: *mut usize,
        parameter_set_count_out: *mut usize,
        nal_unit_header_length_out: *mut c_int,
    ) -> OSStatus;
    pub fn CMBlockBufferCopyDataBytes(
        source_buffer: CMBlockBufferRef,
        offset_to_data: usize,
        data_length: usize,
        destination: *mut c_void,
    ) -> OSStatus;
    pub fn CMVideoFormatDescriptionCreate(
        allocator: CFAllocatorRef,
        codec_type: CMVideoCodecType,
        width: i32,
        height: i32,
        extensions: CFDictionaryRef,
        format_description_out: CMVideoFormatDescriptionRef,
    ) -> OSStatus;
    pub fn CMVideoFormatDescriptionCreateFromHEVCParameterSets(
        allocator: CFAllocatorRef,
        parameter_set_count: usize,
        parameter_set_pointers: *const *const u8,
        parameter_set_sizes: *const usize,
        nal_unit_header_length: i32,
        extensions: CFDictionaryRef,
        format_description_out: CMVideoFormatDescriptionRef,
    ) -> OSStatus;
}

// CoreVideo
#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    pub fn CVPixelBufferCreateWithBytes(
        allocator: CFAllocatorRef,
        width: usize,
        height: usize,
        pixel_format_type: OSType,
        base_address: *mut c_void,
        bytes_per_row: usize,
        release_callback: Option<CVPixelBufferReleaseBytesCallback>,
        release_ref_con: *mut c_void,
        pixel_buffer_attributes: CFDictionaryRef,
        pixel_buffer_out: *mut CVPixelBufferRef,
    ) -> CVReturn;
}
