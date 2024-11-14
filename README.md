# get-mastodon-history (GMH)

This is a small command-line tool to get your Mastodon posts (toots) as one big JSON file.

## Setup

```bash
git clone https://github.com/miikka/get-mastodon-history.git
cargo run -- --help
```

You can pass in the settings via the command-line, use environmental variables, or use `.env` file.

I recommend using `.env` file. Here's a template with the requiredi settings.

```
# The base URL of your Mastodon instance. E.g. https://mastodon.social
GMH_HOST=

# Access token for using Mastodon API
GMH_ACCESS_TOKEN=
```

## Creating a Mastodon application

To use Mastodon's API,

- Go to Mastodon settings -> _Development_ -> _New Application_
  - URL for mastodon.social: https://mastodon.social/settings/applications/new
- Use the following settings:
  - Application name: just put in whatever you like
  - Redirect URI: `urn:ietf:wg:oauth:2.0:oob`
  - Scopes: `read`, `profile`
- Once the application has been created, go to the application details
- Copy the access token and save it to `.env` with `GMH_ACCESS_TOKEN=...`
