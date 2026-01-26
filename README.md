# Rust Shell Implementation

A Unix shell implementation in Rust, supporting basic commands, pipes, and redirections.

## Features

- Execute system commands
- Piping between commands (`ls | grep txt`)
- I/O redirection (`cat file.txt > output.txt`)
- Built-in commands (cd, exit)
- Command history

## Tech Stack

- **Rust 1.75+**
- **std::process** for command execution
- **std::fs** for file operations

## Usage

```bash
cargo run

# In the shell:
$ ls -la
$ echo "hello" | wc -l
$ cat file.txt > output.txt
$ cd /tmp
$ exit
```

## Implementation Highlights

- Tokenizer for parsing command input
- Process spawning and management
- Pipe creation and chaining
- File descriptor handling
- Error handling for system calls
