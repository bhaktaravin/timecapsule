# Timecapsule

Encrypted future messages that unlock on a scheduled date.

Write something today. Share a one-time unlock link with your recipient. The message stays encrypted at rest until `unlock_at`.

## Security model (v1)

- Each capsule is encrypted with a random data key (ChaCha20-Poly1305).
- Data keys are wrapped with a server master key before storage.
- Unlock links use high-entropy tokens; only a SHA-256 hash is stored.
- Message bodies are never logged.

This is **server-side encryption**, not end-to-end. Operational access to the server could decrypt messages. E2E is planned for a later version.

## Requirements

- Rust 1.75+
- OpenSSL not required (pure Rust crypto)

## Setup

```bash
cp .env.example .env
# Edit MASTER_KEY — generate with: openssl rand -hex 32

cargo run
```

Server defaults to `http://127.0.0.1:3000`.

Open that URL in a browser for the web UI:
- **/** — create and seal a message
- **/unlock/** — open a capsule with a token or shared link

### Test unlock (dev mode)

Set `DEV_MODE=1` in `.env`, then on the create page click **Create test unlock**. It seals a sample message that unlocks in 15 seconds, shows a countdown, and links straight to the open page.

```bash
curl -s -X POST http://127.0.0.1:3000/api/dev/test-capsule
```

## API

### Create a capsule

```bash
curl -s -X POST http://127.0.0.1:3000/api/capsules \
  -H 'Content-Type: application/json' \
  -d '{
    "message": "Happy 18th birthday!",
    "recipient_email": "kid@example.com",
    "unlock_at": "2030-06-26T00:00:00Z"
  }'
```

Response includes `unlock_token` and `unlock_path`. **Save the token** — it cannot be recovered.

### Check status

```bash
curl -s http://127.0.0.1:3000/api/capsules/{id}/status
```

### Unlock (recipient)

```bash
curl -s http://127.0.0.1:3000/api/unlock/{token}
```

Returns `403` if the unlock date has not passed or the capsule was already opened.

## Email notifications

When SMTP is configured, the recipient gets an email with the unlock link as soon as the capsule becomes ready.

```env
APP_BASE_URL=https://your-domain.com
SMTP_HOST=smtp.example.com
SMTP_PORT=587
SMTP_USERNAME=your-user
SMTP_PASSWORD=your-password
SMTP_FROM=Timecapsule <noreply@example.com>
```

If `SMTP_HOST` is not set, capsules still unlock normally — only the email step is skipped.

## Project layout

```
src/
  crypto.rs    # envelope encryption + unlock tokens
  db.rs        # SQLite persistence
  routes/      # HTTP handlers
  jobs/        # background unlock scheduler
```

## Roadmap

- [x] Email notification on unlock
- [ ] Video attachments
- [ ] Trusted-contact trigger ("if something happens to me")
- [ ] Optional end-to-end encryption
