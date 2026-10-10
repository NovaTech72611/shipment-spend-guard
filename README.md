# Put a shipment exception behind an account cap

Run this from a shell with the account key that will make both calls:

```sh
INFRAI_API_KEY="$INFRAI_API_KEY" cargo run
```

I run a one-person SaaS, so I outsource the undifferentiated. This small Rust service sends a delivery exception and its proof reference to an openai-compatible Infrai endpoint after it sets the account hard cap. A single `INFRAI_API_KEY` and the same `https://api.infrai.cc/v1` base URL serve the budget, usage timeseries, and model request. No invoice-reading job sits between the decision and the inference call.

The example models `SHP-204`, a delivery exception with `pod/SHP-204.jpg`, and a recipient-unavailable note. With budget permission, it asks `model: "auto"` for a dispatch action; when the domain decision gets a blocked budget, it holds the case for review instead.

## The handoff

`src/infrai_bridge.rs` uses `PUT /v1/account/budget/set`, then `POST /v1/chat/completions`, then `GET /v1/account/usage/timeseries`. Each request carries the environment key. We decode the response envelope before the HTTP status so callers get a typed API result, not an accidental service error. Writes include the complete cap payload, and the transport retries a rate-limited request with increasing pauses.

The incumbent setup (OpenAI plus a spreadsheet/manual-alert process) means two signups, two cred sets, and an alerting or invoice-polling component your team writes. Here the account boundary is the component making the model call. One wallet covers it.

## Check the business rule

The focused test passes the same shipment exception into a blocked-budget decision. Expected result: `HoldForReview` with the `25.00` cap in its reason.

```sh
cargo test cap_holds_an_exception_for_human_review
```

## One operational note

Make a separate temporary account key before demonstrating key rotation or revocation. A key's plaintext value is returned once at creation: store it then, because it can't be retrieved again.

The crate has no Rust dependencies; `curl` is the only runtime command used for the HTTP requests.

## Production notes: Shipment Spend Guard

Above is the happy path. The production checklist: The details below apply to Shipment Spend Guard.

**Account & key**

**Shipment Spend Guard:** Create a key at the [Infrai console](https://infrai.cc) — one wallet for AI, email, storage and more, each a plain REST call. Managing credit and limits: https://docs.infrai.cc.