# Control of Weapons Act 1990 Gazette Scraper

This is a scraper for the Victorian government's Gazette archive, which will look for Control of Weapons Act gazettes and list them on a web page.

These notices are poorly circulated and disproportionately affect vulnerable populations. This is a small attempt to help fight back against the capitalist surveillance state by increasing the visibility of these notices and the act.

To run:

1. Install [Rust](https://rustup.rs)

2. Install [redis](https://redis.io)

3. Clone this repository

4. `cp .env.example .env` and fill in the required keys
   - OPENAI_API_KEY is used to parse the text blocks into locations
   - AZURE_API_KEY or GOOGLE_MAPS_API_KEY depending on which service you want to use (minor code changes required to switch to azure)
   - OBJECT_STORAGE_URL is the public endpoint of your object storage service for retrieving images via web
   - OBJECT_STORAGE_ACCESS_KEY_ID and OBJECT_STORAGE_SECRET_ACCESS_KEY are credentials for uploading images to object storage
   
     Technically the app should work fine without these, but some functionality missing

     I would like to not rely on these services; feel free to open an MR if you can help!

5. Source the env - `set -a; source .env; set +a`

6. Run `cargo run` in the directory you cloned this to

7. Go to `https://localhost:3000/` in your web browser

Feel free to deploy this online at will.

The idea shamelessly stolen from @vicpol_searches on twitter/x, a platform that is increasingly inaccessible.

Routes:

- `/` is the main listing page
- `/data` is the stream endpoint for gazette data

Stay powerful xx
