# Kokoro-82M v1.0 (hexgrad, Apache-2.0), exported for sherpa-onnx by k2-fsa:
# the model, its voices, its tokens, the US English lexicon, and espeak-ng's
# data for words the lexicon doesn't have. Sekka Edu's apps speak with it
# (crates/sekka-voice). The Chinese lexicon, rules and dictionary stay out.
{
  lib,
  stdenvNoCC,
  fetchurl,
}:
stdenvNoCC.mkDerivation {
  pname = "kokoro-multi-lang";
  version = "1.0";
  src = fetchurl {
    url = "https://github.com/k2-fsa/sherpa-onnx/releases/download/tts-models/kokoro-multi-lang-v1_0.tar.bz2";
    hash = "sha256-YHA3B7wFzNSNYOHrcj84sH1uR0eVoLaliNciBADg96Q=";
  };
  dontBuild = true;
  installPhase = ''
    mkdir -p $out
    cp model.onnx voices.bin tokens.txt lexicon-us-en.txt LICENSE README.md $out/
    cp -r espeak-ng-data $out/
  '';
  meta = {
    description = "Kokoro-82M v1.0 text-to-speech for sherpa-onnx";
    homepage = "https://huggingface.co/hexgrad/Kokoro-82M";
    license = lib.licenses.asl20;
    platforms = lib.platforms.all;
  };
}
