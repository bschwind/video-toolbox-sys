use video_toolbox::Encoder;

#[test]
fn test_encode() {
    let width = 1280;
    let height = 720;

    let mut encoder = Encoder::new(width, height).unwrap();

    let src_frame = make_image_frame(width as usize, height as usize);
    let mut dst = vec![0u8; width as usize * height as usize * 4];

    let encoded_size = encoder.encode_blocking(&src_frame, &mut dst).unwrap();

    println!("Encoded size: {}", encoded_size);
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
