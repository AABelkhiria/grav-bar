# grav-bar

Blazing-fast, zero-dependency, and highly customizable custom status line built in Rust for the Google Antigravity CLI (`agy`).

## Features

- **Fast**: Written in pure Rust with zero external dependencies. It completes execution in <1ms!
- **Dynamic Resizing**: Automatically shortens the file path and model names when you resize your terminal window to prevent line wrapping.
- **Quota Parsing**: Automatically parses standard quotas for both Google Gemini and 3rd Party (3p) models.
- **Visual Flourishes**: Supports dynamic colored progress bars for context length and available quotas.
- **Right Aligned Sections**: Calculates terminal width and dynamically aligns agent status and model type to the right edge.

## Installation

### From Source

Clone the repository and build using Cargo:

```sh
git clone https://github.com/AABelkhiria/grav-bar.git
cd grav-bar
cargo build --release
cp target/release/grav-bar ~/.local/bin/grav-bar
```

## Configuration

To use `grav-bar`, configure your Antigravity CLI to pipe its UI JSON payload to the binary.
Add the following to your `~/.gemini/antigravity-cli/settings.json`:

```json
{
  "ui": {
    "status_line": {
      "command": "/Users/YOUR_USER/.local/bin/grav-bar"
    }
  }
}
```

## How It Works

`grav-bar` works by ingesting a raw JSON payload from `stdin` every time the `agy` CLI refreshes its state. It uses manual string parsing to cleanly and efficiently extract only the keys needed (e.g., `cwd`, `display_name`, `terminal_width`) without the overhead of parsing the entire JSON DOM into memory.

## Contributing

Pull requests are welcome! For major changes, please open an issue first to discuss what you would like to change.

Make sure to pass CI by running `cargo fmt` and `cargo clippy -- -D warnings`.

## License

MIT
