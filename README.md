radx
====

An ADX encoder/decoder written in Rust.

Usage
-----

**radx_encode** takes a wav file and encodes it into an adx file.
```
radx_encode [options] INPUT [OUTPUT]

Options:
    -s, --start START   Loop start sample (defaults to song start)
    -e, --end END       Loop end sample (defaults to song end)
    -n, --no-loop       Don't loop the song
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