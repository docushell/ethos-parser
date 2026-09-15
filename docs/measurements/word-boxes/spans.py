"""Span counts a word-span design would reach, against the grounding schema's 1,000,000 cap.
Usage: spans.py GROUNDING.json ..."""
import json, sys
for path in sys.argv[1:]:
    g = json.load(open(path))
    spans = g.get("spans") or []
    words = [len(s["text"].split()) for s in spans]
    every = sum(words)
    multi = sum(n for n in words if n > 1)
    print(path.split("/")[-1], {"run_spans": len(spans), "multi_word_runs": sum(1 for n in words if n > 1),
          "with_a_span_per_word_of_every_run": len(spans) + every,
          "with_a_span_per_word_of_multi_word_runs": len(spans) + multi})
