with import <nixpkgs> {};

runCommand "vision-dev" {
  nativeBuildInputs = [
    pkg-config
    (opencv4.override { enableGtk3 = true; })
  ];
} ''
  cat >$out <<EOF
#!${bash}/bin/bash
export CFLAGS=(-fopenmp $(pkg-config --cflags opencv4))
export LDFLAGS=($(pkg-config --libs opencv4))
export FLAGS=(\$CFLAGS \$LDFLAGS)
EOF
  chmod +x $out
''
