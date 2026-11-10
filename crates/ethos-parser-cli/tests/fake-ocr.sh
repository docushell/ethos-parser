#!/bin/sh
# A recognizer that reads nothing, for the CLI's tests (v3, docs/35-OCR-SCOPE.md §5): it states the
# identity in FAKE_OCR_IDENTITY and answers every request with FAKE_OCR_ANSWER, after reading the
# request's bytes. The real recognizer's protocol is held by ocr/tests/fixtures.rs.
case "$1" in
identity) printf '%s\n' "$FAKE_OCR_IDENTITY" ;;
serve)
    while IFS= read -r header; do
        n=$(printf '%s' "$header" | sed 's/.*"bytes":\([0-9]*\).*/\1/')
        head -c "$n" >/dev/null
        printf '%s\n' "$FAKE_OCR_ANSWER"
    done
    ;;
*) exit 2 ;;
esac
