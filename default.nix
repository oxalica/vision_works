with import <nixpkgs> {};
mkShell {
  buildInputs = [
    pkg-config
    gtk3
    opencl-icd
    cargo
  ];
}
