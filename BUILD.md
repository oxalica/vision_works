# NixOS

1. Make sure you have OpenCL GPU driver and SDK installed (check it with `clinfo`).
2. `cd` to the repository (this directory)
3. Install dependencies and open development shell with:
   ```shell
   nix-shell .
   ```
4. Build & run with:
   ```shell
   cargo run --release --bin img_process
   ```

# Windows

1. Make sure you have OpenCL GPU driver and SDK installed (check it with `clinfo`).
2. Install [`MSYS2`][msys2] for x86_64, and
   run `MSYS2 MinGW 64-bit` from Start Menu to open the shell.
3. Update index and install dependencies:
   ```shell
   pacman -Syy
   pacman -S mingw-w64-x86_64-rust mingw-w64-x86_64-gtk
   ```
4. `cd` to the repository (this directory)
5. build & run with:
   ```shell
   cargo run --release --bin img_process
   ```

[msys2]: https://www.msys2.org/

# Ubuntu

1. Make sure you have OpenCL GPU driver and SDK installed (check it with `clinfo`).
2. Update index and install dependencies:
   ```shell
   sudo apt update
   sudo apt install cargo libgtk-3-dev ocl-icd-opencl-dev
   ```
4. `cd` to the repository (this directory)
5. build & run with:
   ```shell
   cargo run --release --bin img_process
   ```
