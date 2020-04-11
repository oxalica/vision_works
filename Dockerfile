FROM rust:1.42

WORKDIR /build

RUN apt-get update && apt-get install -y \
    libgtk-3-dev libopencv-dev ocl-icd-opencl-dev

VOLUME "/install" 

COPY . .

# Patch for opencv-3.2 on ubuntu
RUN sed -E 's/opencv = "(.*)"/opencv = { version = "\1", default-features = false, features = ["opencv-32"] }/' \
        --in-place ./Cargo.toml && \
    sed -E 's/DrawMatchesFlags::DEFAULT/Default::default()/' \
        --in-place ./src/bin/video_orb/worker.rs

CMD cargo install --path . --root /install
