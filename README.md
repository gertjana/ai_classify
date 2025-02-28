# AI Classify

Aims to have an API to which you can post some text or url or binary and then have an LLM Classify it for you, storing the result so you cn query it later

Aims to support multiple LLM's and multiple storage solutions

# Requirements

* Rust setup
* an openai API key
* A Redis instance running somewhere

Usage:

```shell
cp config.toml.example config.toml
echo "update values in config.toml"
cargo run
```
