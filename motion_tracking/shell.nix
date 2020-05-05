with import <nixpkgs> {};
mkShell rec {
  buildInputs = [
    (python3.withPackages (ps: [
      # opencv4 has memory bug in GOTURN tracker.
      (ps.opencv3.override {
        enableGtk3 = true;
        enableFfmpeg = true;
      })
    ]))
  ];

  keep = linkFarmFromDrvs "keep" buildInputs;
}
