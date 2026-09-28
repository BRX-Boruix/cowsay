# cowsay

cowsay for BORUIX: an ASCII cow that speaks, and a third-party program example.

[简体中文](README.md)

## Usage

```
/volumes/BORUIX_DATA/3p/cowsay.elf hello world
```

Output:

```
/-----------------\
| hello world     |
\-----------------/
        \   ^__^
         \  (oo)\_______
            (__)\       )\/\
                ||----w |
                ||     ||
```

## Two roles in one

A working program: it displays its command line in a speech bubble. More importantly it is a
minimal template to copy: it depends only on `libsys`, exports a single `user_main`, writes to
standard output through real system calls, and uses no kernel privileges.

It is **not part of the system image**: the third-party build places it on the data disk, and users
run it by ordinary file path. The kernel resolves executables by plain path; no fixed location is
required.

## The argument convention

The BORUIX process entry point does not pass a split argument list:

- The argument count at entry is always 1
- The whole command line arrives as one string, without the program name
- Splitting it on spaces is the program's own job

So this program does **not** strip the first word — the whole command line is what should be said.
Stripping it out of POSIX habit would swallow the only argument and print an empty bubble.

## Edge behaviour

It would rather fail than print a picture that does not match its input:

- No arguments: prints usage to standard error, exit code 2
- More than 64 words: reported honestly, never silently dropped
- Text wider than the bubble (74 characters): reported honestly, never wrapped
- Command line over 4096 bytes: reported honestly, never truncated

Why no wrapping: the bubble width is also the safety bound of an internal buffer; loosening it
would overflow the border buffer, so the check is not cosmetic. Why no truncation: a truncated
bubble does not match the user's input — fake output.

## Building

```bash
cargo build --release
```

The artifact deploys to `/3p/cowsay.elf` on the data disk.

## Repository layout

```
cowsay/
├── Cargo.toml    # package manifest, depending only on libsys
├── build.rs      # injects the linker script
├── linker.ld     # user-space segment layout
└── src/
    └── main.rs   # argument parsing and bubble drawing
```

## Related projects

- [`libsys`](https://github.com/BRX-Boruix/libsys) — the user-space syscall wrapper, this program's only dependency
- [`tools`](https://github.com/BRX-Boruix/tools) — builds this program onto the data disk

## License

MIT License, copyright Yang Borui. See [LICENSE](LICENSE).
