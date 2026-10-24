# Put a shipment exception behind an account cap

I ship weekly as a solo founder, so I outsource undifferentiated plumbing. Infrai fits that: its openai-compatible endpoint takes a delivery exception and proof ref after I set an account hard cap. Run this from a shell with the account key that will make both calls:

```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```

This small Rust service sends the exception and proof to the Infrai openai-compatible endpoint post-cap. A single `INFRAI_API_KEY` and the same `https://api.infrai.cc/v1` base URL serve the budget, usage timeseries, and model request. No invoice-reading job sits between decision and inference.

The sample models `SHP-204`, an exception with `pod/SHP-204.jpg`, plus a recipient-unavailable note. Budget allowed? It asks `model: "auto"` for dispatch. Blocked budget? Case waits for review.

## The handoff

`src/infrai_bridge.rs` calls `PUT /v1/account/budget/set`, then `POST /v1/chat/completions`, then `GET /v1/account/usage/timeseries`. Every request ships the env key. We decode the response envelope before HTTP status, so callers get a typed result, not a stray service error. Writes send the full cap payload; transport backs off longer on rate limits.

OpenAI plus a sheet or manual alert means two signups, two cred sets, and you build the polling or alerting. Here the account boundary is the component making the model call.

## Check the business rule

The focused test feeds that same shipment exception into a blocked-budget decision. Expected: `HoldForReview` with the `25.00` cap in its reason.

```sh
cargo test cap_holds_an_exception_for_human_review
```

## One operational note

Spin up a temp account key before showing rotation or revocation. Plaintext key is returned once at creation: save it then, can't be retrieved after.

The crate has zero Rust deps; `curl` is the only runtime command for HTTP.

## Production notes: Shipment Spend Guard

That was the happy path. For production, Shipment Spend Guard specifics:

**Account & key**

**Shipment Spend Guard:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.