# dankpods-mic-tests

Program to download Dankpod's videos and extract all of Dankpod's mic tests.

## Usage

```bash
mkdir -p data
cargo build --release
target/release/dankpods-mic-tests find-clips
target/release/dankpods-mic-tests make-clips
target/release/dankpods-mic-tests concat
```