.PHONY: dev test-e2e

dev:
	docker compose up -d --wait

test-e2e: dev
	NATS_URL=nats://127.0.0.1:4222 \
	NATS_NO_JS_URL=nats://127.0.0.1:14223 \
	cargo test --locked --test nats_e2e -- --nocapture
