radx
====

An ADX encoder/decoder written in Rust.

General Usage
-------------

For most cases where all you want is to decode from adx to wav or encode from
wav to adx, you can just drag the file onto radx and it'll do the conversion for
you without the need to open up a command line. However, you cannot set custom
loop points or specify ahx encoding (which is often used for voice lines)
through the drag and drop interface.

Advanced Usage
--------------

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
