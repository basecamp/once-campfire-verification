# once-campfire-verification

This is the shared verification harness for public Campfire implementations. Keep it independent of any one implementation's checkout or private repositories.

- Fail closed: never count an invalid response, incomplete WebSocket fanout or unverified acknowledged write as throughput.
- Expected message windows and content come from fixture SQL, independently of the server under test.
- Decode real images; content types and magic bytes alone are insufficient. Only the documented ancillary WebP EXIF diagnostic is tolerated, with a real-image regression fixture; decoding failures remain fatal.
- Framework-specific tests stay in their implementation repos. Do not describe shared browser flows as complete parity or as coverage of granted WebPush delivery.
- Keep raw results, generated fixtures, build output and browser artifacts ignored. Publish concise tables only after successful validation.
- Run `bin/check`. Format the standalone Rust client with `cargo fmt --manifest-path loadgen/Cargo.toml`.
- Preserve public source attribution and compatible command-line interfaces when extracting tooling.
