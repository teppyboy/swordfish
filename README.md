# Swordfish

An open-source bot which helps you to choose best swords for fishes around the world.

## Usage

TODO:tm:

## Installation

1. Clone the repository.
2. Install Tesseract
> Tesseract works on Linux way better than Windows, but oh well.
3. Set up your MongoDB database.
> The free tier in MongoDB Atlas is NOT enough as it is limited to 500 entries only.
4. Set up the required environment variables, which contains these variables:
```bash
# Putting all of these into a .env file is fine.
DISCORD_TOKEN=<token>
MONGODB_URL=<mongodb url>
# Optional, only if the url doesn't contain a username.
MONGODB_USERNAME=<mongodb username>
# Optional, only if the url doesn't contain a password.
MONGODB_PASSWORD=<mongodb password>
```
5. Start the bot:
```bash
cargo run
```

## FAQ

### How does it work?

It'd be the same as Nori in general.

## License

[GNU AGPLv3](./LICENSE)

![GNU AGPL](https://www.gnu.org/graphics/agplv3-with-text-162x68.png)
