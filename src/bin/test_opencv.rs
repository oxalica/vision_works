use opencv::{highgui::*, imgcodecs::*};

fn main() -> Result<(), failure::Error> {
    let mat = imread("assets/ddg.png", IMREAD_COLOR)?;
    imshow("test_opencv", &mat)?;
    wait_key(0)?;
    Ok(())
}
