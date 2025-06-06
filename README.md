# Parkera

Simple cloudflare worker that checks a parking queue page and emails
if new spots are listed

## Setup

* Setup cloudflare
* Do wrangler login
* Add secrets
* Run wrangler deploy

Currently using sendgrid, remeber to switch to Amazon SES withing 60 days...

## Testing locally

To test locally first add `.dev.vars` with the secrets then run

``` shell
npx wrangler dev
```

and finally

```shell
curl "http://localhost:8787/cdn-cgi/handler/scheduled"
```

to simulate a cron trigger
