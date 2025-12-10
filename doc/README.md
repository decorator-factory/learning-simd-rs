## Purpose

This somewhat convoluted contraption is my attempt to make a more condensed documentation for Intel intrinsics.

Intel has a decent page called the [Intrinsics Guide](https://www.intel.com/content/www/us/en/docs/intrinsics-guide/index.html),
but it literally lists every single instruction (over 7000) without attempting to group their descriptions.
For example, `_mm256_abs_epi16`, `_mm256_abs_epi8`, and `_mm512_abs_epi8` do essentially the same thing but on differently
sized integers. But they're still listed as separate entries.

I'm not planning to provide the most accurate or complete documentation. If you want to use a function in your program,
you should definitely read the Intel Instrinsics Guide, which this thing links to for every function.
This is more of a quick location and discovery tool. (did you know 52-integers are a thing? now I do, but still
not entirely sure why)

I don't have AVX-512, so I'm only doing this for AVX-2 and earlier.

---

The basic principle is that there's a `./doc.in` file with directives like these:
```
@techs MMX,SSE_ALL,AVX_ALL

# This is a comment

@pattern _mm[256]_add_p{s,d}
Add vectors of f32/f64 elementwise.

@pattern +dpx _mm_add_si64
Silly instruction to add two `u64`/`i64`s. Just use `+`.

@exhaust _mm_.*(?<!lo)add.*
@exhaust-todo .*add.*
```

This file can be validated and converted to HTML with `doc.htmlbuild`. These directives are supported:

- `@techs` selects which "techs" (`MMX,SSE_ALL,AVX_ALL,AVX_512,AMX,SVML,Other`) to load
- `@pattern` starts a new paragraph describing the selected intrinsics.
    - In the first example, it's going to select `_mm_add_ps`, `_mm_add_pd`, `_mm256_add_ps`, `_mm256_add_pd`
    - In the second example, it's going to select only `_mm_add_si64` and also mark it as "deprecated". It's not actually
    deprecated, but you probably have better ways to add two 64-bit numbers; it's also not in `core::arch::x86` in Rust.
- `@exhaust` with a regular expression ([Python3 flavor](https://docs.python.org/3/library/re.html#regular-expression-syntax))
    asserts that the patterns so far cover every intrinsic in the selected techs matching the pattern.
    In this case we want to ensure that we cover all the instructions concerned with addition (and
    excluding `_mm_loaddup_pd` which happens to contain the substring "add").
- `@exhaust-todo` is like `@exhaust`, but produces a warning instead of an error

## Instructions:

### To update `intrinsics.glob`

1) Download the XML data file from Intel (the current URL is `https://www.intel.com/content/dam/develop/public/us/en/include/intrinsics-guide/data-3-6-9.xml`, might update in the future; it's not included here because it's huge)
2) Have Python 3.12 or later installed
3) Run `python3 -m doc.intbuild path/to/data.xml > ./doc/intrinsics.glob`

If you have the offline version of Intel's intrinsics guide, it has this XML embedded
as a JS string in the `data.js` file (WTF, Intel?). You'll need to extract it yourself.

### To create `out.html`

`python3 -m doc.htmlbuild > out.html`

You can optionally set the `INTEL_BASE_URL` environment variable if you want to use a different root
for links to the Intel Intrinsics Guide (e.g. use the offline version).
