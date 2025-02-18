{
  pkgs ? import <nixpkgs> {},
  lib ? pkgs.lib,
}:
pkgs.mkShell rec {
  buildInputs = with pkgs; [
    libxkbcommon
    wayland
    pkg-config
  ];

  shellHook = ''
    export LD_LIBRARY_PATH="$LD_LIBRARY_PATH:${lib.makeLibraryPath buildInputs}";
  '';
}
