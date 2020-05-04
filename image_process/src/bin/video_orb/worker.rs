use super::GuiEvent;
use opencv::{
    core::Scalar,
    features2d::{draw_keypoints, DrawMatchesFlags},
    imgproc::{cvt_color, COLOR_BGR2RGB},
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

            // GTK expects RGB colorspace.
            let mut detected_rgb = Mat::default()?;
            cvt_color(&detected, &mut detected_rgb, COLOR_BGR2RGB, 0)?;

            // If we still have time remained, sleep until the next frame.
            let now = Instant::now();
            if now < expect_show_inst {
                thread::sleep(expect_show_inst - now);
            }

            if stop_guard.strong_count() == 0 {
                break;
            }
            if tx.send(GuiEvent::Frame(detected_rgb)).is_err() {
                // Main loop exited.
                break;
            }
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
