{
  lib,
  mkShell,
  cargo,
  rustfmt,
  clippy,
  taplo,
  pkg-config,
  wayland,
  libxkbcommon,
}: let
  runtimeDeps = [
    libxkbcommon
    wayland
  ];
in
  mkShell {
    name = "rust";

    strictDeps = true;
    nativeBuildInputs = [
      pkg-config

      cargo
      clippy
      (rustfmt.override {asNightly = true;})
      taplo
    ];

    buildInputs = runtimeDeps;

    env.LD_LIBRARY_PATH = "$LD_LIBRARY_PATH:${lib.makeLibraryPath runtimeDeps}";
  }
