# derive-agent-tools workspace

This workspace hosts the crates that make up the `derive-agent-tools` project.

- `derive-agent-tools` – the public crate users depend on. It re-exports the
  procedural macros and exposes the runtime helpers used in the macro expansion.
- `derive-agent-tools-macros` – the proc-macro implementation crate. It is an
  internal dependency that is published alongside the main crate but not used
  directly by consumers.

See `derive-agent-tools/README.md` for detailed usage documentation, examples,
and feature flags.
