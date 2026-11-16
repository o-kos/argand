# Packaging fixture

`iq-2k.flac` is a synthetic stereo PCM16 I/Q recording at 2000 samples/second,
4096 complex samples long. I is a 125 Hz cosine and Q a 125 Hz sine, each with
peak amplitude 2048 integer units (1/16 full scale). No recorded user data is used.

The WAV source was generated with Python's `wave` and `struct` modules, rounding
`2048 * cos(2*pi*125*n/2000)` and `2048 * sin(2*pi*125*n/2000)` for integer
`n` from 0 through 4095. It was converted with SoX 14.4.2 using dithering disabled:

```sh
sox -D iq-2k.wav iq-2k.flac
```

Shared release packaging renders this file with packaged aspec to check stereo
FLAC input without requiring a GPU. This fixture is released under the repository's
MIT licence.

`LICENSE-LUCIDE` is an unmodified test copy from locked `gpui-kit-assets` 0.9.1
(crate SHA-256 `cb5abe0bbd8f9226170c8726e529f6574519e297e9ed1bde86d5fd4d3536079a`).
It preserves the ISC Lucide and MIT Feather notices. Archive tests check both
attributions and the separately shipped DejaVu font notice.
