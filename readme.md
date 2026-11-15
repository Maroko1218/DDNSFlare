# DDNSFlare
If you have Cloudflare as your DNS provider, you can generate an API key to update/create zones when the IP address changes

# Quickstart (No Docker)
1. Make a file called `api_key.env` containing your cloudflare api key.
2. run the program with `./ddnsflare <path_to_api_key.env> <domain.name.to.keep.updated>`

# Quickstart (With Docker)
1. Make a file called `api_key.env` containing your cloudflare api key.
2. Edit the final line of `Dockerfile` to point to the `domain.name.to.keep.updated`
3. Run `docker compose up -d`