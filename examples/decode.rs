use core::ffi::c_void;
use core_foundation::{
    base::{CFIndexConvertible, OSStatus},
    boolean::CFBoolean,
    dictionary::{
        kCFTypeDictionaryKeyCallBacks, kCFTypeDictionaryValueCallBacks, CFDictionaryCreate,
    },
    string::CFStringRef,
};
use std::convert::TryInto;
use video_toolbox_sys::{
    kVTVideoDecoderSpecification_RequireHardwareAcceleratedVideoDecoder, CMTime,
    CMVideoFormatDescriptionCreateFromHEVCParameterSets, CMVideoFormatDescriptionRef,
    CVImageBufferRef, VTDecodeInfoFlags, VTDecompressionSessionCreate, VTDecompressionSessionRef,
};

extern "C" fn decode_callback(
    _output_callback_ref_con: *mut c_void,
    _source_frame_ref_con: *mut c_void,
    _status: OSStatus,
    _info_flags: VTDecodeInfoFlags,
    _image_buffer: CVImageBufferRef,
    _presentation_timestamp: CMTime,
    _presentation_duration: CMTime,
) {
    // println!("decode_callback");

    // println!("Status: {}", status);

    // println!("Valid buffer: {}", unsafe { CMSampleBufferIsValid(sample_buffer) });
    // // Returns the total size in bytes of sample data in a CMSampleBuffer.
    // let data_length = unsafe { CMSampleBufferGetTotalSampleSize(sample_buffer) };
    // println!("Total sample size: {}", data_length);

    // let data_buffer = unsafe { CMSampleBufferGetDataBuffer(sample_buffer) };
    // println!("Data buffer: {:?}", data_buffer);

    // let format = unsafe { CMSampleBufferGetFormatDescription(sample_buffer) };

    // let vps = get_hevc_param(format, HevcParam::Vps).unwrap();
    // let sps = get_hevc_param(format, HevcParam::Sps).unwrap();
    // let pps = get_hevc_param(format, HevcParam::Pps).unwrap();

    // let mut hevc_data = vec![0u8; data_length];

    // let offset = 0;
    // let _ = unsafe {
    //     CMBlockBufferCopyDataBytes(
    //         data_buffer,
    //         offset,
    //         data_length,
    //         hevc_data.as_mut_ptr() as *mut _,
    //     )
    // };

    // const HEADER: &[u8; 4] = &[0, 0, 0, 1];

    // let mut output = vec![];
    // output.extend_from_slice(HEADER);
    // output.extend_from_slice(&vps);

    // output.extend_from_slice(HEADER);
    // output.extend_from_slice(&sps);

    // output.extend_from_slice(HEADER);
    // output.extend_from_slice(&pps);

    // let mut buffer_offset = 0;

    // while buffer_offset < (hevc_data.len() - HEADER.len()) {
    //     let mut nal_len = u32::from_ne_bytes([
    //         hevc_data[buffer_offset],
    //         hevc_data[(buffer_offset + 1)],
    //         hevc_data[(buffer_offset + 2)],
    //         hevc_data[(buffer_offset + 3)],
    //     ]);
    //     nal_len = u32::from_be(nal_len);
    //     dbg!(nal_len);

    //     output.extend_from_slice(HEADER);
    //     let hevc_offset = buffer_offset + HEADER.len();
    //     output.extend_from_slice(&hevc_data[hevc_offset..(hevc_offset + nal_len as usize)]);

    //     buffer_offset += HEADER.len();
    //     buffer_offset += nal_len as usize;
    // }

    // std::mem::forget(vps);
    // std::mem::forget(sps);
    // std::mem::forget(pps);

    // std::fs::write("out.hevc", &output).unwrap();

    // unsafe {
    //     if let Some(custom_val) = (source_frame_ref_con as *mut u32).as_mut() {
    //         *custom_val = 37;
    //     }
    // }

    // dbg!(hevc_data.len());
}

fn main() {
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
        // CMVideoFormatDescriptionCreate(
        //     std::ptr::null(),       // Allocator
        //     kCMVideoCodecType_HEVC, // codec type
        //     frame_width as i32,     // width
        //     frame_height as i32,    // height
        //     std::ptr::null(),       // extensions
        //     format_ref.as_mut_ptr() as CMVideoFormatDescriptionRef,
        // );

        // TODO(bschwind) - Hardcoded for now, but extract from the HEVC stream later
        let vps: Vec<u8> = vec![
            64, 1, 12, 1, 255, 255, 1, 96, 0, 0, 3, 0, 176, 0, 0, 3, 0, 0, 3, 0, 150, 21, 192, 144,
        ];
        let sps: Vec<u8> = vec![
            66, 1, 1, 1, 96, 0, 0, 3, 0, 176, 0, 0, 3, 0, 0, 3, 0, 150, 160, 2, 128, 128, 45, 22,
            32, 87, 185, 22, 85, 53, 2, 2, 2, 164, 2,
        ];
        let pps: Vec<u8> = vec![68, 1, 192, 44, 188, 20, 201];

        let parameter_set_sizes = vec![vps.len(), sps.len(), pps.len()];
        let parameter_sets = vec![vps.as_ptr(), sps.as_ptr(), pps.as_ptr()];

        CMVideoFormatDescriptionCreateFromHEVCParameterSets(
            std::ptr::null(),     // Allocator
            parameter_sets.len(), // parameter set count
            parameter_sets.as_ptr(),
            parameter_set_sizes.as_ptr(),
            4,                // NAL unit header length
            std::ptr::null(), // extensions
            format_ref.as_mut_ptr() as CMVideoFormatDescriptionRef,
        );

        let format = format_ref.assume_init();

        format
    };

    // https://github.com/peter-iakovlev/TelegramUI/blob/e8b193443d1b84f00390138a82c44ebfcceb496a/TelegramUI/FFMpegMediaFrameSourceContextHelpers.swift#L67-L92
    // https://stackoverflow.com/questions/29525000/how-to-use-videotoolbox-to-decompress-h-264-video-stream/29525001#29525001

    // Create the decoder
    let mut decompression_ref = std::mem::MaybeUninit::<VTDecompressionSessionRef>::uninit();

    let create_status = unsafe {
        VTDecompressionSessionCreate(
            std::ptr::null(),      // Allocator
            format_description,    // Format Description
            decoder_specification, // Decoder specification,
            std::ptr::null(),      // Dest image buffer attributes
            Some(decode_callback), // Output callback, pass NULL if you're using VTDecompressionSessionDecodeFrameWithOutputHandler
            decompression_ref.as_mut_ptr() as VTDecompressionSessionRef,
        )
    };

    if create_status != 0 {
        println!("Failed to create VT Compression Session: {}", create_status);
        return;
    }

    let _compression_session = unsafe { decompression_ref.assume_init() };

    // // Create the frame to encode
    // // let mut frame_data = vec![0u8; (frame_width * frame_height * 4) as usize];
    // let frame_data = make_image_frame(frame_width, frame_height);

    // println!("Uncompressed size: {}", frame_data.len());

    // let mut pixel_buffer_ref = std::mem::MaybeUninit::<CVPixelBufferRef>::uninit();
    // let k_cvpixel_format_type_32_argb = 0x00000020; // TODO(bschwind) - get this from CoreVideo
    // let pixel_buffer_create_status = unsafe {
    //     CVPixelBufferCreateWithBytes(
    //         std::ptr::null(),
    //         frame_width as usize,
    //         frame_height as usize,
    //         k_cvpixel_format_type_32_argb,
    //         frame_data.as_ptr() as *mut c_void,
    //         (4 * frame_width) as usize, // bytes per row
    //         None,
    //         std::ptr::null_mut(),
    //         std::ptr::null(),
    //         pixel_buffer_ref.as_mut_ptr() as *mut CVPixelBufferRef,
    //     )
    // };

    // if pixel_buffer_create_status != 0 {
    //     println!("Failed to create Pixel Buffer: {}", pixel_buffer_create_status);
    //     return;
    // }

    // let pixel_buffer = unsafe { pixel_buffer_ref.assume_init() };

    // println!("Got a pixel buffer, good to go!");

    // let frame_time = CMTime { value: 0i64, timescale: 1i32, flags: 0u32, epoch: 0i64 };

    // let invalid_duration = CMTime { value: 0i64, timescale: 0i32, flags: 0u32, epoch: 0i64 };

    // let mut custom_val = 0u32;

    // let encode_start = std::time::Instant::now();
    // // Encode the frame
    // let encode_status = unsafe {
    //     VTCompressionSessionEncodeFrame(
    //         compression_session,
    //         pixel_buffer,
    //         frame_time,                                 // Presentation timestamp
    //         invalid_duration,                           // Frame duration
    //         std::ptr::null(),                           // Frame Properties
    //         &mut custom_val as *mut u32 as *mut c_void, // Source frame ref con
    //         std::ptr::null_mut(),                       // Info flags out
    //     );
    // };

    // println!("Encode status: {:?}", encode_status);

    // // Wait for the encode to finish.
    // let _ = unsafe {
    //     VTCompressionSessionCompleteFrames(compression_session, invalid_duration);
    // };

    // println!("Took: {:?}", encode_start.elapsed());
    // println!("Our custom value is {}", custom_val);
}
