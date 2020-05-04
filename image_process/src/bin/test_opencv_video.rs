use opencv::videoio::{VideoCapture, VideoCaptureTrait as _, CAP_FFMPEG};

fn main() {
    let file_name = std::env::args().nth(1).expect("Missing argument");
    let cap = VideoCapture::from_file(&file_name, CAP_FFMPEG).unwrap();
    assert!(cap.is_opened().unwrap(), "Cannot open file");
    println!("Success");
}
