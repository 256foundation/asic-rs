# Documentation Workflow

This project has three documentation targets:

| Target | Source | Output |
| --- | --- | --- |
| Zensical site | `docs/index.md`, `docs/getting-started.md`, `docs/api.md` | `site/` |
| Rust crate docs and root README | `docs-shared/guide.md` included by `src/lib.rs` | docs.rs and `README.md` |
| Python package README | `docs-shared/guide.md` referenced by `pyproject.toml` | PyPI package description |

Keep user-facing pages focused on using the library. Build and generation
instructions belong here.

## Authoring Rules

Use Zensical content tabs in site pages when showing equivalent Rust and Python
examples:

````markdown
=== "Rust"

    ```rust
    let miner = factory.get_miner(ip).await?;
    ```

=== "Python"

    ```python
    miner = await factory.get_miner("192.168.1.10")
    ```
````

Keep `docs-shared/guide.md` in plain Markdown. It is included directly in
Rustdoc and is also the Python package README, so it should avoid Zensical-only
syntax.

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
python -m pip install -e ".[docs]"
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
