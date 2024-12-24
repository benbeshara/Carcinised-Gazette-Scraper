# Control of Weapons Act 1990 Gazette Scraper

This is a scraper for the Victorian government's Gazette archive, which will look for Control of Weapons Act gazettes and list them on a web page.

These notices are poorly circulated and disproportionately affect vulnerable populations. This is a small attempt to help fight back against the capitalist surveillance state by increasing the visibility of these notices and the act.

To run:

1. Install [Rust](https://rustup.rs)

2. Install [redis](https://redis.io)

3. Clone this repository

4. Run `cargo run` in the directory you cloned this to

5. Go to `https://localhost:3000/` in your web browser

Feel free to deploy this online at will.

The idea shamelessly stolen from @vicpol_searches on twitter/x, a platform that is increasingly inaccessible.

Routes:

- `/` is the main listing page
- `/data` is the stream endpoint for gazette data

Stay powerful xx
