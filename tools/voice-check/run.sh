#!/usr/bin/env bash
# Say each line of corpus.txt with espeak-ng (as Anzan used to) and with
# Kokoro (af_heart), have Whisper write down what it heard, and score it
# (DEVELOPERS.md, "The voice"). Takes a folder to work in, and needs:
#   KOKORO   Kokoro's model folder (nix build .#kokoro)
#   SHERPA   sherpa-onnx's bin folder (sherpa-onnx-offline-tts)
#   WHISPER  whisper.cpp's whisper-cli
#   MODEL    a Whisper model, e.g. ggml-base.en.bin
# and espeak-ng on PATH.
set -euo pipefail
here="$(cd "$(dirname "$0")" && pwd)"
out="${1:?usage: run.sh FOLDER}"
mkdir -p "$out/wav"
i=0
while IFS= read -r line; do
  i=$((i + 1))
  n=$(printf %02d $i)
  espeak-ng -v en-us+f3 -s 135 -p 62 -w "$out/wav/es-$n.wav" "$line"
  "$SHERPA/sherpa-onnx-offline-tts" --kokoro-model="$KOKORO/model.onnx" --kokoro-voices="$KOKORO/voices.bin" \
    --kokoro-tokens="$KOKORO/tokens.txt" --kokoro-data-dir="$KOKORO/espeak-ng-data" \
    --kokoro-lexicon="$KOKORO/lexicon-us-en.txt" --num-threads=2 --sid=3 --kokoro-length-scale=1.08 \
    --output-filename="$out/wav/ko-$n.wav" "$line" > /dev/null 2>&1
done < "$here/corpus.txt"
for f in "$out"/wav/*.wav; do
  printf '%s\t%s\n' "$(basename "$f" .wav)" "$("$WHISPER" -m "$MODEL" -nt -np -t 8 -f "$f" 2> /dev/null | tr '\n' ' ')"
done > "$out/heard.tsv"
cd "$out" && cp "$here/corpus.txt" . && python3 "$here/score.py" "$@"
