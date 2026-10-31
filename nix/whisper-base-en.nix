# Whisper base.en (OpenAI, MIT) as whisper.cpp converts it: the model Fumi
# listens with in Ondoku. The revision and SHA-256 are Koe's own catalogue's
# (desktop/aizome/apps/koe/data/models.json in Sekka).
{ lib, fetchurl }:
fetchurl {
  name = "ggml-base.en.bin";
  url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base.en.bin";
  sha256 = "a03779c86df3323075f5e796cb2ce5029f00ec8869eee3fdfb897afe36c6d002";
  meta = {
    description = "Whisper base.en for whisper.cpp";
    homepage = "https://huggingface.co/ggerganov/whisper.cpp";
    license = lib.licenses.mit;
  };
}
