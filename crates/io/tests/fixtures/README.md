`levels.flac` is a synthetic mono 48 kHz PCM16 FLAC fixture: 16384 samples,
repeating [0.125, -0.25, 0.5, -0.75] 4096 times. It was encoded with libsndfile.
Tests clear the STREAMINFO sample count to exercise unknown-length decoding.
