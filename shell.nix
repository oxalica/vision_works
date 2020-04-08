with import <nixpkgs> {};
mkShell {
  buildInputs = [
    cargo
    pkg-config
    gtk3
    opencl-icd
    (opencv4.override {
      enableGtk3 = true;
      enableFfmpeg = true;
    })
  ];
}
