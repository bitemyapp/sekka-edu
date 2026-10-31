# Sekka Edu's apps, built against Aizome's kit and Sukumo (`aizome`: the
# kit's part of Sekka's desktop/aizome), laid out beside this tree as the
# workspace's path dependency `../sekka/desktop/aizome` expects.
{
  lib,
  rustPlatform,
  runCommand,
  makeWrapper,
  fontconfig,
  wayland,
  pipewire,
  espeak-ng,
  src,
  aizome,
  fonts,
}:
let
  source = runCommand "sekka-edu-source" { } ''
    mkdir -p $out/sekka/desktop
    cp -r ${src} $out/sekka-edu
    cp -r ${aizome} $out/sekka/desktop/aizome
  '';
in
rustPlatform.buildRustPackage {
  pname = "sekka-edu";
  version = "0.1.0";
  src = source;
  sourceRoot = "sekka-edu-source/sekka-edu";
  cargoLock.lockFile = ../Cargo.lock;
  nativeBuildInputs = [ makeWrapper ];
  # Pictures in the tests are drawn with the session's type.
  AIZOME_FONT_DIR = fonts;
  LD_LIBRARY_PATH = lib.makeLibraryPath [
    fontconfig
    wayland
  ];
  postInstall = ''
    for app in Anzan Kids; do
      dir=apps/$(echo $app | tr A-Z a-z)
      install -Dm644 $dir/data/computer.sekka.$app.desktop $out/share/applications/computer.sekka.$app.desktop
      install -Dm644 $dir/data/computer.sekka.$app.svg $out/share/icons/hicolor/scalable/apps/computer.sekka.$app.svg
    done
    # Anzan plays its sounds with PipeWire and Pon speaks with espeak-ng.
    wrapProgram $out/bin/sekka-anzan \
      --suffix PATH : ${
        lib.makeBinPath [
          pipewire
          espeak-ng
        ]
      } \
      --prefix LD_LIBRARY_PATH : ${
        lib.makeLibraryPath [
          fontconfig
          wayland
        ]
      } \
      --set-default AIZOME_FONT_DIR ${fonts}
    wrapProgram $out/bin/sekka-kids \
      --prefix LD_LIBRARY_PATH : ${
        lib.makeLibraryPath [
          fontconfig
          wayland
        ]
      } \
      --set-default AIZOME_FONT_DIR ${fonts}
  '';
  meta = {
    description = "Sekka Edu: Anzan, the mental abacus, and Sekka Kids, profiles and reports for grown-ups";
    homepage = "https://github.com/bitemyapp/sekka-edu";
    license = with lib.licenses; [
      mit
      asl20
    ];
    platforms = lib.platforms.linux;
    mainProgram = "sekka-anzan";
  };
}
