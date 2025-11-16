# ChatGPT CLI

A lightweight, colorful terminal client for OpenAI’s GPT models. It keeps the running conversation in memory, renders markdown nicely (headings, bullet points, code blocks, bold, italics), and now speaks directly to the new [`/v1/responses`](https://platform.openai.com/docs/api-reference/responses) endpoint with optional reasoning effort controls.

## Requirements

- Rust 1.80+ (install via [rustup](https://rustup.rs/))
- An OpenAI API key exported as `OPENAI_API_KEY`
- Network access to `api.openai.com`

## Building

```bash
git clone https://github.com/<your-account>/chatgpt-cli.git
cd chatgpt-cli
cargo build --release
```

The binary will be available at `target/release/chatgpt-cli`.

## Running

```bash
export OPENAI_API_KEY="sk-your-key"
cargo run -- --reasoning low
```

### Installing globally

To call the CLI from any directory as `chatgpt`:

```bash
cargo install --path . --bin chatgpt
```

Make sure `$HOME/.cargo/bin` is on your `PATH` (rustup adds it automatically). After that simply run:

```bash
chatgpt
```

Flags:

- `--reasoning <none|low|medium|high>` – optional (defaults to `none`). If omitted the CLI starts with reasoning disabled, but you can still change it at runtime using the `/reasoning` command.
- `-h`/`--help` – show usage.

## In-session commands

Once the CLI is running you can type:

- `/new` – start a fresh conversation (clears the buffer sent to OpenAI).
- `/exit` – quit the CLI.
- `/reasoning <none|low|medium|high>` – change reasoning effort on the fly. Typing `/reasoning` without arguments shows the current level.

## Tips

- The interface prints a reminder banner every time it starts. If you need to silence colors for scripting environments, run the CLI inside a terminal that strips ANSI codes.
- When something goes wrong the CLI prints descriptive error messages (e.g., invalid arguments or missing API key). Fix the highlighted issue and relaunch.

Happy prompting! ✨
