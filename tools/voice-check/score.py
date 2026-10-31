# Word error rate of what Whisper heard, per voice, against corpus.txt (run.sh).
# `-v` prints every line as each voice was heard.
import re, sys
ones = "zero one two three four five six seven eight nine ten eleven twelve thirteen fourteen fifteen sixteen seventeen eighteen nineteen".split()
tens = "_ _ twenty thirty forty fifty sixty seventy eighty ninety".split()
def say(n):
    if n < 20: return ones[n]
    if n < 100: return tens[n // 10] + ("" if n % 10 == 0 else " " + ones[n % 10])
    return ones[n // 100] + " hundred" + ("" if n % 100 == 0 else " " + say(n % 100))
def norm(t):
    t = t.lower().replace("-", " ").replace("…", " ")
    t = re.sub(r"\d+", lambda m: say(int(m.group())), t)
    return re.sub(r"[^a-z' ]", " ", t).split()
def dist(a, b):
    d = list(range(len(b) + 1))
    for i, x in enumerate(a, 1):
        p, d[0] = d[0], i
        for j, y in enumerate(b, 1):
            p, d[j] = d[j], min(d[j] + 1, d[j - 1] + 1, p + (x != y))
    return d[-1]
lines = [l.rstrip("\n") for l in open("corpus.txt") if l.strip()]
heard = dict(l.rstrip("\n").split("\t", 1) for l in open("heard.tsv"))
for eng, name in (("es", "espeak-ng en-us+f3"), ("ko", "Kokoro af_heart")):
    errs = words = exact = 0
    for i, line in enumerate(lines, 1):
        want, got = norm(line), norm(heard.get(f"{eng}-{i:02d}", ""))
        e = dist(want, got)
        errs += e; words += len(want); exact += e == 0
    print(f"{name}: {errs}/{words} words wrong, WER {100*errs/words:.1f}%, {exact}/{len(lines)} lines heard exactly")
if "-v" in sys.argv:
    for i, line in enumerate(lines, 1):
        print(f"{line!r}\n  espeak: {heard.get(f'es-{i:02d}','').strip()!r}\n  kokoro: {heard.get(f'ko-{i:02d}','').strip()!r}")
