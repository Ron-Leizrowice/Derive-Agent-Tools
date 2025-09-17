# derive-agent-tools-macros

Internal implementation crate for the `derive-agent-tools` procedural macros.
Consumers should depend on the `derive-agent-tools` crate instead, which
re-exports these macros together with the runtime support items required by the
expansions.

This crate is published so that `derive-agent-tools` can depend on it on crates.io,
but its API surface is not considered stable by itself.
