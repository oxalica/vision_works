with import <nixpkgs> {};
mkShell rec {
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

  keep = linkFarmFromDrvs "keep" buildInputs;
}
