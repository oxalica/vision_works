with import <nixpkgs> {};
mkShell rec {
  buildInputs = [
    (python3.withPackages (ps: [
      (ps.opencv3.override {
        enableGtk3 = true;
        enableFfmpeg = true;
      })
    ]))
  ];

  keep = linkFarmFromDrvs "keep" buildInputs;
}
