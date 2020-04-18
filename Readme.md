# Zpng_rs

![](https://github.com/benmkw/zpng_rs/workflows/CI/badge.svg)

this is ported from https://github.com/catid/Zpng as a fun evening activity

it does not (yet) work on stable cause it uses const generics

```man
zpng_rs --help
Usage: target/debug/zpng_rs [-c] [-d] [--test] -i <inpath> [-o <outpath>]

Zpng_rs - Experimental Lossless Image Compressor

Options:
  -c, --compress    compress an image (jpeg, webp, tga, bmp, png, gif, ico),
                    saves as .zpng
  -d, --decompress  decompress a .zpng image, saves as .png
  --test            test the compressor for compatibility with input file. 1st:
                    Makes sure that it can decompress the image without writing
                    it to disc. 2nd: Outputs zpng by itself and by calling the
                    original zpng tool. 3rd: Decompresses the foreign output and
                    lets the original zpng tool decompress its own output. You
                    can adapt the path in the source to test your
                    implementation.
  -i, --inpath      input file
  -o, --outpath     output file, deduced to be the input filename with .png
  --help            display usage information

```

## Ideas/ Future Work
#### (i may not work on them in the short term, they serve as memory for me or ideas for you)
- deduce output filetype and offer other options than png, (all the ones which are used for input)
- compile to wasm possible? maybe use in https://github.com/benmkw/svelte_rust_test
    - need to try the (slower) pure rust implementation of zstd for wasm target https://github.com/gyscos/zstd-rs/issues/48#issuecomment-427916136
- add timing / throughput (input, output) information when using verbose flag (first add verbose flag)
- benchmark
- better CLI interface, maybe use clap (gives linker error/ investigate) because it has mutually exclusive options
- write nicer (top level) doc comments such that cargo doc is interesting (but its very easy so its just for learning cargo docs properly)
- work on size of binary, rust will probably be bigger but there are ways to mitigate this, I already reduced features of the image loading lib

- experiment with different filtering approaches and try to increase compression
- plot the tradeoff between zstd compression settings size and time over a representative image dataset

## I want to highlight some points that come to my mind after porting this:

Like the original author my timeframe was set to one evening and thus take the following for what its worth. (Actually this turned into three evenings after debugging and working on it some more)

**It's important for me to emphasize that this was not a rir (rewrite in rust) because I thought the original implementation was bad/ slow or something like that.**
It was meant to exercise my understanding of rust/ use some libraries (zstd and image) and have a reason to read the code more in depth.

Memory management in the original code was arguably more robust then in my rust port.
The original code allocates memory manually and checks the return value whereas my rust port uses things like `vec![default_value; size]` which just panics if memory is exhausted, or the program gets stopped by the OS.
Zig has some more opinions on that https://ziglang.org/.

The cpp version has less dependencies but it can also be argued that the rust version supports more image formats (I believe) and has a slightly nicer CLI because it uses a crate to generate the help text and do the parsing.

In the cpp code templates are used which I emulated in the rust version by enabling const generics. The cpp code also uses template specialization which I replaced with cfg! macros. It should also be possible by using rusts experimental specialization features but they are more experimental than const generics and I did not try them here yet.

It would be interesting to profile the code. I looked at the assembly and from a quick glance it seemed as if the rust code did not generate SIMD instructions yet which is a bit unfortunate but I don't know yet if the cpp does.

The Rust version could make use of macros to unroll the loops that depend on the constant generic parameter and it would be interesting to see how this affects codegen/ see if this currently already happens.

The cpp version is closer to c than modern cpp and I don't mean this in a negative way. It uses output parameters. It makes use of computing known sizes and avoiding reallocations by allocating upper bounds in advance and thus not relying on dynamic data structures such as vector. I may be using zstd a little less efficiently in my rust code.

The decision to represent a buffer as a vector instead of a pointer and a length could be made in cpp as well (or be made in rust differently as well) and is a decision that the author made so its not a difference between rust and cpp but a matter of personal preference.

For low level manipulations of the header-data, the cpp code casts a pointer into the buffer to a pointer to a header struct which is common practice but sadly undefined behavior. I did not use `mem::transmute` in rust but rather picked the bytes by hand which is not a great solution either. Using yet another crate for this would be possible, I believe zerocopy would solve such a problem.

It was very easy to build the cpp code which made it much quicker to debug my version and make it compatible with the original. I had a bug in big/ little endian encoding/ decoding which I found by cross testing.

The cpp code vendors its dependencies while rust can make use of cargo. (Zig has some more opinions on that as well https://ziglang.org/, zig package manager, zig as a cross compiler)

The rust code could be made more typesafe by replacing some values with enums because they can only take a small number of values but they would make the code (much) more verbose so its a tradeoff.

I would encourage you to rewrite this in your favorite language or a new language you want to learn more about.
This project has several nice properties

- its short
- using c libraries with a c interface like zstd
- using an iamge loading library (or just use stb_image) so this should be possible in almost any language
- doing low level byte manipulations for writing the header of the file format (see how some language handels this)


Lastly I want to thank the original author Christopher A. Taylor for sharing his work and being very responsive.

## Appendix, or what is this project worth?
```sh
$ scc src/ --by-file
───────────────────────────────────────────────────────────────────────────────
Language                 Files     Lines   Blanks  Comments     Code Complexity
───────────────────────────────────────────────────────────────────────────────
Rust                         2       645      110        61      474         36
───────────────────────────────────────────────────────────────────────────────
src/lib.rs                           447       83        39      325         27
src/bin/main.rs                      198       27        22      149          9
───────────────────────────────────────────────────────────────────────────────
Total                        2       645      110        61      474         36
───────────────────────────────────────────────────────────────────────────────
Estimated Cost to Develop $12,335
Estimated Schedule Effort 2.887570 months
Estimated People Required 0.506040
───────────────────────────────────────────────────────────────────────────────
```
We can see that the code that does the actual work in lib.rs is only 325 loc, not bad for an image format I'd say.

Of course there are the dependencies but this is kind of the point, to quote the original author:

> The goal was to see if I could create a better lossless compressor than PNG in just one evening (a few hours) using Zstd and some past experience writing my GCIF library. Zstd is magical.

> I'm not expecting anyone else to use this, but feel free if you need some fast compression in just a few hundred lines of C code.

Btw. Arch has switched to using zstd for its packages as well.

## Your code is not functional enough !!!1!

```rust
fn PackAndFilter<const kChannels: usize>(
    input: &[u8],
    width: u16,
    _height: u16,
    _byteCount: usize,
) -> Vec<u8> {
    input
        .chunks(width as usize * kChannels as usize)
        .map(|row| {
            row.chunks(kChannels)
                .scan([0; kChannels], |prev, channel| {
                    Some(prev.iter_mut().zip(channel).map(|(prev_i, curr)| {
                        let d = curr.wrapping_sub(*prev_i);
                        *prev_i = *curr;
                        d
                    }))
                })
                .flatten()
        })
        .flatten()
        .collect::<Vec<u8>>()
}
```
I'd like to see the resulting assembly but I did not manage to settle my dispute with the compiler yet. There is also a concern that this might (re)allocate the resulting buffer and thus result in slower code.
If you know how to solve this I'm very interested.

## License
Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in this crate by you, shall be licensed as defined in the BSD 3-Clause license, without any additional terms or conditions.
