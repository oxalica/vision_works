with import <nixpkgs> {};
mkShell {
  buildInputs = [
    pkg-config
    (opencv4.override { enableGtk3 = true; })
    gtk3
    opencl-icd
  ];
}
