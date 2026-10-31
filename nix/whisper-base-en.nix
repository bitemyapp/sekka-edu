# Whisper base.en (OpenAI, MIT) as whisper.cpp converts it, its weights
# quantized to 8 bits: the model Fumi listens with in Ondoku. It heard
# readings as well as the full-size model, in half the time, and is half
# the size (DEVELOPERS.md, "Hearing faster"). The revision is Koe's own
# catalogue's (desktop/aizome/apps/koe/data/models.json in Sekka); the
# SHA-256 is Hugging Face's for the file at that revision.
{ lib, fetchurl }:
fetchurl {
  name = "ggml-base.en-q8_0.bin";
  url = "https://huggingface.co/ggerganov/whisper.cpp/resolve/5359861c739e955e79d9a303bcbc70fb988958b1/ggml-base.en-q8_0.bin";
  sha256 = "a4d4a0768075e13cfd7e19df3ae2dbc4a68d37d36a7dad45e8410c9a34f8c87e";
  meta = {
    description = "Whisper base.en for whisper.cpp, quantized to 8 bits";
    homepage = "https://huggingface.co/ggerganov/whisper.cpp";
    license = lib.licenses.mit;
  };
}
