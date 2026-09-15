# Documentation Workflow

This project has three documentation targets:

| Target | Source | Output |
| --- | --- | --- |
| Zensical site | `docs/index.md`, `docs/getting-started.md`, `docs/api.md` | `site/` |
| Rust crate docs and root README | `docs-shared/guide.md` included by `src/lib.rs` | docs.rs and `README.md` |
| Python package README | `docs-shared/guide.md` referenced by `python/pyasic-rs/pyproject.toml` | PyPI package description |

Keep user-facing pages focused on using the library. Build and generation
instructions belong here.

## Authoring Rules

Use Zensical content tabs in site pages when showing equivalent Rust, Python,
and Go examples:

````markdown
=== "Rust"

    ```rust
    let miner = factory.get_miner(ip).await?;
    ```

=== "Python"

    ```python
    miner = await factory.get_miner("192.168.1.10")
    ```

=== "Go"

    ```go
    miner, err := factory.GetMiner("192.168.1.10")
    ```
````

Keep `docs-shared/guide.md` in plain Markdown. It is included directly in
Rustdoc and is also the Python package README, so it should avoid Zensical-only
syntax.

Go packaging and cgo notes live in `go/README.md`. Rebuild the FFI artifacts
with `make -C go ffi` before running Go tests.

## Binding Layout

Language bindings live in their language directories:

| Binding | Native source | Package source |
| --- | --- | --- |
| Python | `python/pyasic-rs/src` | `python/pyasic-rs/pyasic_rs` |
| C ABI | `c/asic-rs-ffi/src` | `c/asic-rs-ffi/include` (generated) |
| Go | Uses the C ABI crate | `go/asic_go` |

The Python package owns its Cargo manifest, build script, Python configuration,
and tests. Run `python scripts/run_python_tests.py` from the repository root to
build the extension and run its tests. Go retains its published module path;
`make -C go test` builds the C ABI and runs the Go tests.

Generate Python type stubs from the repository root:

```sh
uvx maturin generate-stubs --manifest-path python/pyasic-rs/Cargo.toml --features python -o python/pyasic-rs
```

Maturin adds the `pyasic_rs` package directory to the output path, writing
`python/pyasic-rs/pyasic_rs/asic_rs.pyi`. The pre-commit stub hook uses the same
command. Release wheels and source archives go to the root `dist/` directory.

## Regenerate The Root README

Regenerate the root README from Rust crate docs:

```sh
cargo +nightly doc2readme --expand-macros --template README.j2 > README.md
```

The nightly toolchain is needed because `src/lib.rs` includes `docs-shared/guide.md`
with `include_str!`, and `cargo-doc2readme` needs macro expansion to read it.

## Preview The Zensical Site

Install the documentation extra in your active environment if needed:

```sh
python -m pip install -e "./python/pyasic-rs[docs]"
```

Preview while editing:

```sh
zensical serve
```

Build the static site:

```sh
zensical build
```

The site configuration lives in `zensical.toml`.
