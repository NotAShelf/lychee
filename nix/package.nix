{
  lib,
  rustPlatform,
}:
rustPlatform.buildRustPackage {
  pname = "lychee";
  version = "1.0.0";

  src = let
    fs = lib.fileset;
    s = ../.;
  in
    fs.toSource {
      root = s;
      fileset = fs.unions [
        (s + /crates)
        (s + /Cargo.lock)
        (s + /Cargo.toml)
      ];
    };

  cargoLock.lockFile = ../Cargo.lock;
  enableParallelBuilding = true;

  meta = {
    description = "Simple, opinionated image viewer for Wayland";
    maintainers = with lib.maintainers; [NotAShelf];
    license = lib.licenses.mpl20;
  };
}
