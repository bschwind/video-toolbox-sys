#![cfg(any(target_os = "macos", target_os = "ios"))]
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use core_foundation::{
    array::CFArrayRef,
    base::{Boolean, CFAllocatorRef, CFIndex, OSStatus},
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
pub type VTDecodeFrameFlags = u32;

// CoreMedia Types
pub type FourCharCode = u32;
pub type OSType = FourCharCode;
pub type CMVideoCodecType = FourCharCode;
pub type CMBlockBufferFlags = u32;
pub type CMItemCount = CFIndex;

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
pub type CVOptionFlags = u64;

pub const kCVPixelBufferLock_ReadOnly: CVOptionFlags = 0x00000001;
pub const kCVPixelFormatType_32BGRA: u32 = fourcc(b"BGRA");

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

#[repr(C)]
pub struct VTDecompressionOutputCallbackRecord {
    pub decompression_output_callback: Option<VTDecompressionOutputCallback>,
    pub decompression_output_ref_con: *mut c_void,
}

pub type VTDecompressionOutputCallback = extern "C" fn(
    output_callback_ref_con: *mut c_void,
    source_frame_ref_con: *mut c_void,
    status: OSStatus,
    info_flags: VTDecodeInfoFlags,
    image_buffer: CVImageBufferRef,
    presentation_timestamp: CMTime,
    presentation_duration: CMTime,
);

pub type CMSampleBufferMakeDataReadyCallback = extern "C" fn(
    sample_buffer: CMSampleBufferRef,
    make_data_ready_ref_con: *mut c_void,
) -> OSStatus;

// Core Graphics

#[repr(C)]
#[derive(Debug)]
pub struct CGSize {
    width: f64,
    height: f64,
}

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
        output_callback: *const VTDecompressionOutputCallbackRecord,
        decompression_session_out: VTDecompressionSessionRef,
    ) -> OSStatus;

    pub fn VTDecompressionSessionDecodeFrame(
        session: VTDecompressionSessionRef,
        sample_buffer: CMSampleBufferRef,
        decode_flags: VTDecodeFrameFlags,
        source_frame_ref_con: *const c_void,
        info_flags_out: *mut VTDecodeInfoFlags, // TODO - is it mutable?
    ) -> OSStatus;

    pub fn VTDecompressionSessionWaitForAsynchronousFrames(
        session: VTDecompressionSessionRef,
    ) -> OSStatus;
}

// CoreMedia
#[link(name = "CoreMedia", kind = "framework")]
extern "C" {
    pub static kCMSampleAttachmentKey_DisplayImmediately: CFStringRef;
    pub static kCMSampleAttachmentKey_NotSync: CFStringRef;

    pub fn CMSampleBufferIsValid(sample_buffer: CMSampleBufferRef) -> Boolean;
    pub fn CMSampleBufferGetTotalSampleSize(sample_buffer: CMSampleBufferRef) -> usize;
    pub fn CMSampleBufferGetDataBuffer(sample_buffer: CMSampleBufferRef) -> CMBlockBufferRef;
    pub fn CMSampleBufferGetFormatDescription(
        sample_buffer: CMSampleBufferRef,
    ) -> CMFormatDescriptionRef;
    pub fn CMSampleBufferCreate(
        allocator: CFAllocatorRef,
        data: CMBlockBufferRef,
        data_ready: bool,
        make_data_ready_callback: Option<CMSampleBufferMakeDataReadyCallback>,
        make_data_ready_ref_con: *mut c_void,
        format_description: CMFormatDescriptionRef,
        num_samples: CMItemCount,
        num_sample_timing_entries: CMItemCount,
        sample_timing_array: *const c_void, // Actually a CMSampleTimingInfo
        num_sample_size_entry: CMItemCount,
        sample_size_array: *const usize,
        sample_buffer_out: CMSampleBufferRef,
    ) -> OSStatus;
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
    pub fn CMBlockBufferCreateWithMemoryBlock(
        allocator: CFAllocatorRef,
        memory_block: *const c_void,
        block_length: usize,
        block_allocator: CFAllocatorRef,
        custom_block_source: *const c_void, // Pointer to CMBlockBufferCustomBlockSource
        offset_to_data: usize,
        data_length: usize,
        flags: CMBlockBufferFlags,
        block_buffer_out: CMBlockBufferRef,
    ) -> OSStatus;
    pub fn CMSampleBufferGetSampleAttachmentsArray(
        sample_buffer: CMSampleBufferRef,
        create_if_necessary: bool,
    ) -> CFArrayRef;
}

// CoreVideo
#[link(name = "CoreVideo", kind = "framework")]
extern "C" {
    pub static kCVPixelBufferPixelFormatTypeKey: CFStringRef;
    pub static kCVPixelBufferIOSurfacePropertiesKey: CFStringRef;

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
    pub fn CVImageBufferGetEncodedSize(buffer: CVImageBufferRef) -> CGSize;
    pub fn CVImageBufferGetDisplaySize(buffer: CVImageBufferRef) -> CGSize;
    pub fn CVPixelBufferGetDataSize(buffer: CVImageBufferRef) -> usize;
    pub fn CVPixelBufferGetWidth(buffer: CVImageBufferRef) -> usize;
    pub fn CVPixelBufferGetHeight(buffer: CVImageBufferRef) -> usize;
    pub fn CVPixelBufferGetPixelFormatType(buffer: CVImageBufferRef) -> OSType;
    pub fn CVPixelBufferLockBaseAddress(buffer: CVImageBufferRef, flags: CVOptionFlags)
        -> CVReturn;
    pub fn CVPixelBufferUnlockBaseAddress(
        buffer: CVImageBufferRef,
        flags: CVOptionFlags,
    ) -> CVReturn;
    pub fn CVPixelBufferGetBaseAddress(buffer: CVImageBufferRef) -> *const c_void;

    // Planar Functions
    pub fn CVPixelBufferIsPlanar(buffer: CVImageBufferRef) -> bool;
    pub fn CVPixelBufferGetPlaneCount(buffer: CVImageBufferRef) -> usize;
    pub fn CVPixelBufferGetBaseAddressOfPlane(
        buffer: CVImageBufferRef,
        plane_index: usize,
    ) -> *const c_void;
    pub fn CVPixelBufferGetBytesPerRowOfPlane(
        buffer: CVPixelBufferRef,
        plane_index: usize,
    ) -> usize;
    pub fn CVPixelBufferGetHeightOfPlane(buffer: CVPixelBufferRef, plane_index: usize) -> usize;
}
