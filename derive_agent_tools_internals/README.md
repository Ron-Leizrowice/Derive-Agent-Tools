# derive_agent_tools_internals

Internal implementation crate for the `derive_agent_tools` procedural macros.
Consumers should depend on the `derive_agent_tools` crate instead, which
re-exports these macros together with the runtime support items required by the
expansions.

This crate is published so that `derive_agent_tools` can depend on it on crates.io,
but its API surface is not considered stable by itself.
