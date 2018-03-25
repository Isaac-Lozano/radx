radx
====

An ADX encoder/decoder written in Rust.

Written by OnVar.

Download
--------

Download the latest version from the [releases page](https://github.com/Isaac-Lozano/radx/releases).

Usage
-----

**radx_encode** takes a wav file and encodes it into an adx file.
```
radx_encode [options] INPUT [OUTPUT]

Options:
    -s, --start START   Loop start sample (defaults to song start)
    -e, --end END       Loop end sample (defaults to song end)
    -n, --no-loop       Don't loop the song
    -a, --ahx           Use ahx encoding (cannot loop)
    -h, --help          Print this help menu
```

**radx_decode** takes an adx file and decodes it into a wav file.
```
radx_decode [options] INPUT [OUTPUT]

Options:
    -l, --loop N        Loop N times
    -i, --info          Print adx header info
    -h, --help          Print this help menu
```
