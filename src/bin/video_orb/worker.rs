use super::{Frame, GuiEvent};
use opencv::{
    core::Scalar,
    features2d::{draw_keypoints, DrawMatchesFlags},
    prelude::*,
    types::VectorOfKeyPoint,
    videoio::VideoCapture,
};
use std::{
    sync::Weak,
    thread,
    time::{Duration, Instant},
};

// Worker thread for ORB.
pub fn worker(
    mut cap: VideoCapture,
    fps: f64,
    tx: glib::Sender<GuiEvent>,
    stop_guard: Weak<()>,
) -> Result<(), failure::Error> {
    let mut frame = Mat::default()?;
    let mut orb = ORB::default()?;

    let start_inst = Instant::now();
    let mut frame_idx = 0u64;

    while cap.read(&mut frame)? {
        frame_idx += 1;
        let expect_show_inst = start_inst + Duration::from_secs_f64(frame_idx as f64 / fps);

        // If not timeout, run the detector. Otherwise, skip frames to catch up.
        if Instant::now() < expect_show_inst {
            let detected = detect_features(&frame, &mut orb)?;
            let out = convert_img(detected)?;

            // If we still have time remained, sleep until the next frame.
            let now = Instant::now();
            if now < expect_show_inst {
                thread::sleep(expect_show_inst - now);
            }

            if stop_guard.strong_count() == 0 {
                break;
            }
            tx.send(GuiEvent::Frame(out))?;
        }
    }

    Ok(())
}

fn detect_features(img: &Mat, detector: &mut impl Feature2DTrait) -> opencv::Result<Mat> {
    let mut keypoints = VectorOfKeyPoint::new();
    detector.detect(img, &mut keypoints, &Mat::default()?)?;
    let mut img_out = Mat::default()?;
    draw_keypoints(
        &img,
        &keypoints,
        &mut img_out,
        Scalar::all(-1.0),
        DrawMatchesFlags::DEFAULT,
    )?;
    Ok(img_out)
}

fn convert_img(mat: Mat) -> Result<Frame, failure::Error> {
    use opencv::core::Vec3b;

    let (height, width) = (mat.rows() as usize, mat.cols() as usize);
    let row_stride = width * 3;
    let mut data = vec![0u8; height * width * 3];

    for x in 0..height {
        for y in 0..width {
            let idx = (x * width + y) * 3;
            let [b, g, r] = mat.at_2d::<Vec3b>(x as i32, y as i32)?.0;
            data[idx + 0] = r;
            data[idx + 1] = g;
            data[idx + 2] = b;
        }
    }

    Ok(Frame {
        height,
        width,
        row_stride,
        data,
    })
}
