# Sekka Edu's apps, built against Aizome's kit and Sukumo (`aizome`: the
# kit's part of Sekka's desktop/aizome, and Koe), laid out beside this tree
# as the workspace's path dependency `../sekka/desktop/aizome` expects.
{
  lib,
  rustPlatform,
  runCommand,
  makeWrapper,
  cmake,
  openblas,
  fontconfig,
  wayland,
  pipewire,
  espeak-ng,
  sherpa-onnx,
  andika,
  kokoro,
  whisper,
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
  readingFont = "${andika}/share/fonts/truetype/Andika-Regular.ttf";
  libraryPath = lib.makeLibraryPath [
    fontconfig
    wayland
  ];
in
rustPlatform.buildRustPackage {
  pname = "sekka-edu";
  version = "0.1.0";
  src = source;
  sourceRoot = "sekka-edu-source/sekka-edu";
  cargoLock.lockFile = ../Cargo.lock;
  nativeBuildInputs = [
    makeWrapper
    # Koe's whisper.cpp, built by whisper-rs-sys through CMake.
    cmake
  ];
  buildInputs = [ openblas ];
  # Ondoku listens with Koe's whisper.cpp, with OpenBLAS, which picks kernels
  # for the processor at run time while the rest stays portable (no -march):
  # Sekka's own settings for Koe (desktop/aizome/nix/package.nix).
  # whisper-rs-sys passes GGML_* to CMake, which Cargo runs: this isn't a
  # CMake project.
  buildFeatures = [ "sekka-ondoku/openblas" ];
  dontUseCmakeConfigure = true;
  WHISPER_DONT_GENERATE_BINDINGS = "1";
  BLAS_INCLUDE_DIRS = "${lib.getDev openblas}/include";
  OPENBLAS_PATH = "${lib.getLib openblas}";
  GGML_NATIVE = "OFF";
  GGML_AVX = "OFF";
  GGML_AVX2 = "OFF";
  GGML_FMA = "OFF";
  GGML_F16C = "OFF";
  # Pictures in the tests are drawn with the session's type, and Ondoku's
  # pages with Andika.
  AIZOME_FONT_DIR = fonts;
  SEKKA_EDU_READING_FONT = readingFont;
  LD_LIBRARY_PATH = libraryPath;
  postInstall = ''
    for app in Anzan Kids Ondoku; do
      dir=apps/$(echo $app | tr A-Z a-z)
      install -Dm644 $dir/data/computer.sekka.$app.desktop $out/share/applications/computer.sekka.$app.desktop
      install -Dm644 $dir/data/computer.sekka.$app.svg $out/share/icons/hicolor/scalable/apps/computer.sekka.$app.svg
    done
    # Anzan plays its sounds with PipeWire. Pon speaks in Kokoro through
    # sherpa-onnx (crates/sekka-voice), or with espeak-ng if that fails.
    wrapProgram $out/bin/sekka-anzan \
      --set-default SEKKA_EDU_KOKORO ${kokoro} \
      --set-default SEKKA_EDU_SHERPA_ONNX ${sherpa-onnx}/lib/libsherpa-onnx-c-api.so \
      --suffix PATH : ${
        lib.makeBinPath [
          pipewire
          espeak-ng
        ]
      } \
      --prefix LD_LIBRARY_PATH : ${libraryPath} \
      --set-default AIZOME_FONT_DIR ${fonts}
    # Ondoku listens through PipeWire's pw-record with Whisper base.en, speaks
    # as Anzan does, and sets its pages in Andika.
    wrapProgram $out/bin/sekka-ondoku \
      --set-default SEKKA_EDU_KOKORO ${kokoro} \
      --set-default SEKKA_EDU_SHERPA_ONNX ${sherpa-onnx}/lib/libsherpa-onnx-c-api.so \
      --set-default SEKKA_EDU_WHISPER ${whisper} \
      --set-default SEKKA_EDU_READING_FONT ${readingFont} \
      --suffix PATH : ${
        lib.makeBinPath [
          pipewire
          espeak-ng
        ]
      } \
      --prefix LD_LIBRARY_PATH : ${libraryPath} \
      --set-default AIZOME_FONT_DIR ${fonts}
    # Sekka Kids speaks only to let a grown-up hear how a name is said.
    wrapProgram $out/bin/sekka-kids \
      --set-default SEKKA_EDU_KOKORO ${kokoro} \
      --set-default SEKKA_EDU_SHERPA_ONNX ${sherpa-onnx}/lib/libsherpa-onnx-c-api.so \
      --suffix PATH : ${
        lib.makeBinPath [
          pipewire
          espeak-ng
        ]
      } \
      --prefix LD_LIBRARY_PATH : ${libraryPath} \
      --set-default AIZOME_FONT_DIR ${fonts}
  '';
  meta = {
    description = "Sekka Edu: Anzan, the mental abacus; Ondoku, a reading buddy; and Sekka Kids, profiles and reports for grown-ups";
    homepage = "https://github.com/bitemyapp/sekka-edu";
    license = with lib.licenses; [
      mit
      asl20
    ];
    platforms = lib.platforms.linux;
    mainProgram = "sekka-anzan";
  };
}
