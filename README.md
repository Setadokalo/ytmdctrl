# YTMDCTRL
### A Command Line Tool to Control YTMD

This project provides a way to control YTMD from the command line, or inside of shell scripts.


## Building

1. [Install Rust](https://www.rust-lang.org/tools/install)

2. Download this repository

3. Run `cargo build --release`

4. The tool should now be built and in the `target/release/` folder.

## How to Use

When running the tool for the first time, it will request an authorization token from YTMD. Once approved, all further runs with the same server will not require reauthorization. However, note that different ways to refer to the same server will behave unexpectedly - connecting with the ip `localhost`, then `127.0.0.1`, will request authorization again, and then a subsequent `localhost` connection will fail due to it's authorization token having been overwritten on the server's side.

The tool has a built-in help function, which lists all available commands and how to use them.