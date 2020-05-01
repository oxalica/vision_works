
## Runtime dependencies

### Note

**Run the executable from this directory!**
It will read `glade/*` at runtime

### Requirement
- opencl
- gtk3
- opencv3.2

### Dependency installation (Ubuntu)
1. Install OpenCL SDK and drivers
2. Run:
    ```
    sudo apt-get update
    sudo apt-get install \
        libgtk-3-0 \
        libopencv-shape3.2 libopencv-stitching3.2 libopencv-superres3.2 libopencv-videostab3.2 \
        libopencv-video3.2 libopencv-calib3d3.2 libopencv-features2d3.2 libopencv-flann3.2 \
        libopencv-objdetect3.2 libopencv-ml3.2 libopencv-videoio3.2 libopencv-imgcodecs3.2 \
        libopencv-photo3.2 libopencv-imgproc3.2 libopencv-core3.2
    ```
