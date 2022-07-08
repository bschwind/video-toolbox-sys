use video_toolbox::Decoder;

#[test]
fn test_decode() {
    let width = 1280;
    let height = 720;
    let hevc_bytes = include_bytes!("../../video-toolbox-sys/out.hevc");

    let mut decoder = Decoder::new(width, height).unwrap();
    let mut dst = vec![0u8; width as usize * height as usize * 4];

    let decoded_size = decoder.decode_blocking(hevc_bytes, &mut dst).unwrap();

    println!("Decoded size: {}", decoded_size);
}
