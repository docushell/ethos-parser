#!/bin/sh
# An assist binary that asks no server, for the CLI's tests (decision #149, assist/README.md): it
# states the identity in FAKE_ASSIST_IDENTITY; on `ask SERVER FILE PAGES [MODEL] <PDF` it writes its
# argv to FAKE_ASSIST_LOG, the PDF it read to FAKE_ASSIST_LOG.pdf and the key it was given to
# FAKE_ASSIST_LOG.key, copies FAKE_ASSIST_REPLIES to FILE when set, says FAKE_ASSIST_SAYS on
# stderr, and exits FAKE_ASSIST_EXIT. With FAKE_ASSIST_NOISE, it first writes that many bytes to
# stderr, before reading the PDF. The real binary's protocol is held by assist/tests/.
case "$1" in
identity) printf '%s\n' "$FAKE_ASSIST_IDENTITY" ;;
ask)
    printf '%s\n' "$@" >"$FAKE_ASSIST_LOG"
    if [ -n "${FAKE_ASSIST_NOISE-}" ]; then
        head -c "$FAKE_ASSIST_NOISE" /dev/zero | tr '\0' x >&2
        echo >&2
    fi
    cat >"$FAKE_ASSIST_LOG.pdf"
    printf '%s' "${ETHOS_PARSER_ASSIST_KEY-}" >"$FAKE_ASSIST_LOG.key"
    if [ -n "${FAKE_ASSIST_REPLIES-}" ]; then cp "$FAKE_ASSIST_REPLIES" "$3"; fi
    if [ -n "${FAKE_ASSIST_SAYS-}" ]; then printf '%s\n' "$FAKE_ASSIST_SAYS" >&2; fi
    exit "${FAKE_ASSIST_EXIT:-0}"
    ;;
*) exit 2 ;;
esac
